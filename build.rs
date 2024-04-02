#![allow(dead_code)]

use std::{borrow::Borrow, ffi::OsStr, path::{Path, PathBuf}, process::Command};
use ::bindgen::{Builder, CodegenConfig, EnumVariation};
use embuild::{bindgen::{self}, cargo, cmake::{self, file_api::{codemodel::Language, ObjKind}, Query}};
use walkdir::WalkDir;
use anyhow::{bail, Context, Error};

#[allow(unused_macros)]
macro_rules! p {
	($($tokens: tt)*) => {
		println!("cargo:warning={}", format!($($tokens)*))
	}
}

pub fn get_compiler(target: &str) -> String {
	let mut compiler = cc::Build::new();
 	let compiler = compiler
		.target(target)
		.get_compiler();
  	let compiler = compiler
		.path();

	compiler.to_str().expect(&format!("Failed to find compiler for target '{}'", target)).to_string()
}

pub fn get_sysroot(target: &str) -> Result<String, anyhow::Error> {
	let mut sysroot = cc::Build::new();
	let sysroot = sysroot
		.target(target)
		.get_compiler()
		.to_command()
		.arg("--print-sysroot")
		.output()
		.context("Couldn't find target GCC executable.")
		.and_then(|output| {
			if output.status.success() {
				Ok(String::from_utf8(output.stdout)?)
			} else {
				panic!("Couldn't read output from GCC.")
			}
		})?;

	Ok(sysroot.trim().to_string())
}

pub fn get_cpp_headers(compiler: &str) -> Result<Vec<PathBuf>, anyhow::Error> {
	let mut cpp_headers = cc::Build::new();
	cpp_headers
		.cpp(true)
		.no_default_flags(true)
		.compiler(compiler)
		.get_compiler()
		.to_command()
		.arg("-E")
		.arg("-Wp,-v")
		.arg("-xc++")
		.arg(".")
		.output()
		.context("Couldn't find target GCC executable.")
		.and_then(|output| {
			// We have to scrape the gcc console output to find where
			// the c++ headers are. If we only needed the c headers we
			// could use `--print-file-name=include` but that's not
			// possible.
			let gcc_out = String::from_utf8(output.stderr)?;

			// Scrape the search paths
			let search_start = gcc_out.find("search starts here").unwrap();
			let search_paths: Vec<PathBuf> = gcc_out[search_start..]
				.split('\n')
				.map(|p| PathBuf::from(p.trim()))
				.filter(|path| path.exists())
				.collect();

			Ok(search_paths)
		})
}

pub fn cmake_build_project(target: &str, source_dir: &str) -> PathBuf {
	cmake::Config::new(source_dir)
		.generator("Unix Makefiles")
		.target(target)
		.define("CMAKE_C_FLAGS", "-mcpu=cortex-m0plus -mthumb")
		.define("TF_LITE_STRIP_ERROR_STRINGS", "true")
		.build_target("hello_world")
		.always_configure(true)
		.build()
}

pub fn create_bindgen(query: Query<'_>, sysroot: &str, cpp_headers: Vec<PathBuf>, target_platform: &str) -> Result<Builder, Error> {
	let replies = query.get_replies()?;
	let target = replies
		.get_codemodel()?
		.into_first_conf()
		.get_target("hello_world")
		.unwrap_or_else(|| {
			bail!("Could not read build information from cmake: Target 'hello_world.elf' not found")
		})?;

	let compiler = replies
		.get_toolchains()
		.and_then(|mut t| {
			t.take(Language::C)
				.ok_or_else(|| Error::msg("No C toolchain"))
		})
		.and_then(|t| {
			t.compiler
				.path
				.ok_or_else(|| Error::msg("No compiler path set"))
		})
		.context("Could not determine the compiler from cmake")?;

	let binder = bindgen::Factory::from_cmake(target.compile_groups.get(0).expect("Failed to get compile group"))?
		.with_linker(&compiler)
		.with_sysroot(sysroot);

	let mut binder = ::bindgen::Builder::default().clang_args(binder.clang_args);

	for include in cpp_headers {
		binder = binder.clang_arg(format!("-I{}", include.into_os_string().into_string().unwrap()));
	}

	let binder = binder
		.clang_arg(format!("--target={}", target_platform))
		.clang_arg(format!("-isysroot {}", "/opt/homebrew/Cellar/arm-none-eabi-gcc/10.3-2021.07/gcc/arm-none-eabi"))
		.clang_arg(format!("--sysroot={}", "/opt/homebrew/Cellar/arm-none-eabi-gcc/10.3-2021.07/gcc/arm-none-eabi"))
		.clang_arg(format!("-mcpu=cortex-m0plus"))
		.clang_arg(format!("-mthumb"))
		.clang_arg(format!("-I/submodules/pico-tflmicro/src/tensorflow"))
		.derive_eq(true)
		.use_core()
		.clang_arg("-xc++")
		.clang_arg("-std=c++11")
		.allowlist_recursively(true)
		.prepend_enum_name(false)
		.impl_debug(true)
		.with_codegen_config(CodegenConfig::TYPES)
		.layout_tests(false)
		.enable_cxx_namespaces()
		.derive_default(true)
		.size_t_is_usize(true)
		.use_core()
		.ctypes_prefix("cty")
		// Types
		.allowlist_type("tflite::MicroErrorReporter")
		.opaque_type("tflite::MicroErrorReporter")
		.allowlist_type("tflite::Model")
		.opaque_type("tflite::Model")
		.allowlist_type("tflite::MicroInterpreter")
		.opaque_type("tflite::MicroInterpreter")
		.allowlist_type("tflite::ops::micro::AllOpsResolver")
		.opaque_type("tflite::ops::micro::AllOpsResolver")
		.allowlist_type("TfLiteTensor")
		.allowlist_type("FrontendState")
		.allowlist_type("FrontendConfig")
		.allowlist_type("FrontendOutput")
		// Types - blacklist
		.blocklist_type("std")
		.blocklist_type("tflite::Interpreter_TfLiteDelegatePtr")
		.blocklist_type("tflite::Interpreter_State")
		.default_enum_style(
			EnumVariation::Rust {
				non_exhaustive: false,
			})
		.derive_partialeq(true)
		.derive_eq(true)
		.detect_include_paths(false)
		.parse_callbacks(Box::new(::bindgen::CargoCallbacks::new()))
		.emit_clang_ast();
	Ok(binder)
}

// Taken from https://github.com/Recognition2/tfmicro under apache license
fn run_command_or_fail<P, S>(dir: &str, cmd: P, args: &[S])
where
    P: AsRef<Path>,
    S: Borrow<str> + AsRef<OsStr>,
{
    let cmd = cmd.as_ref();
    let cmd = if cmd.components().count() > 1 && cmd.is_relative() {
        // If `cmd` is a relative path (and not a bare command that should be
        // looked up in PATH), absolutize it relative to `dir`, as otherwise the
        // behavior of std::process::Command is undefined.
        // https://github.com/rust-lang/rust/issues/37868
        PathBuf::from(dir)
            .join(cmd)
            .canonicalize()
            .expect("canonicalization failed")
    } else {
        PathBuf::from(cmd)
    };
    eprintln!(
        "Running command: \"{} {}\" in dir: {}",
        cmd.display(),
        args.join(" "),
        dir
    );
    let ret = Command::new(cmd).current_dir(dir).args(args).status();
    match ret.map(|status| (status.success(), status.code())) {
        Ok((true, _)) => {}
        Ok((false, Some(c))) => panic!("Command failed with error code {}", c),
        Ok((false, None)) => panic!("Command got killed"),
        Err(e) => panic!("Command failed with error: {}", e),
    }
}

pub fn get_cmake_query() -> Result<Query<'static>, Error> {
	let out_dir = cargo::out_dir();

	let cmake_build_dir = out_dir.join("build");

	cmake::Query::new(
		&cmake_build_dir,
		"cargo",
		&[ObjKind::Codemodel, ObjKind::Toolchains, ObjKind::Cache],
	)
}

fn generate_and_output_bindings() -> Result<(), Error> {
	if !Path::new("submodules/pico-tflmicro/README.md").exists() {
        eprintln!("Setting up submodules");
        run_command_or_fail(".", "git", &["submodule", "update", "--init"]);
    }

	let target = "thumbv6m-none-eabi";
	let pico_tflmicro_dir = "submodules/pico-tflmicro".to_string();

	let compiler = get_compiler(&target);
	let sysroot = get_sysroot(&target)?;
	let dst = cmake_build_project(&target, &pico_tflmicro_dir);
	let cpp_headers = get_cpp_headers(&compiler)?;
	let query = get_cmake_query()?;

	println!("cargo:rustc-link-search=native={}/build", dst.display());
	println!("cargo:rustc-link-lib=static=pico-tflmicro");

	let headers = WalkDir::new(pico_tflmicro_dir + "/src/tensorflow")
		.into_iter()
		.filter_map(|e| e.ok())
		.filter(|e| {
			// Filter out directories named "internal"
			let path = e.path().to_str().unwrap();
			let is_file = e.file_type().is_file();
			let is_not_test = is_file && !path.contains("test");
			is_not_test && e.path().extension().map_or(false, |ext| ext == "h")
		})
		.map(|e| e.path().to_str().unwrap().to_owned())
		.collect::<Vec<String>>();


	let mut binder = create_bindgen(query, &sysroot, cpp_headers, &target)?;
	for header in headers {
		binder = binder.header(header);
	}

	let _ = binder.generate()?.write_to_file("src/bindings.rs");
	Ok(())
}

fn main() -> Result<(), Error> {
	#[cfg(feature = "build")]
	generate_and_output_bindings()?;
	
	#[cfg(not(feature = "build"))]
	{
		let binary = Path::new("prebuilt").to_str().unwrap();
		println!("cargo:warning=Feature 'build' is disabled. Prebuilt binary and bindings will be used.");
		println!("cargo:rustc-link-search=native={}", binary);
		println!("cargo:rustc-link-lib=static=pico-tflmicro");
	}

	Ok(())
}

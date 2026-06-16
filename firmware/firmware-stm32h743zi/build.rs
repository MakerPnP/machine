use std::env;
use std::fs::File;
use std::io::Write;
use std::path::PathBuf;

#[derive(Copy, Clone)]
enum Linker {
    Llvm,
    Gcc,
}

fn main() {
    let encoded_rustflags = env::var("CARGO_ENCODED_RUSTFLAGS");
    let linker = match encoded_rustflags {
        Ok(flags) if flags.contains("linker=arm-none-eabi-gcc") => Linker::Gcc,
        _ => Linker::Llvm,
    };

    println!("cargo:rerun-if-changed=memory.x");

    configure_bin_linker_scripts(linker);
    configure_bin_linker_options(linker);

}

fn configure_bin_linker_scripts(_linker: Linker) {
    // Put `memory.x` in our output directory and ensure it's on the linker search path.
    let out = &PathBuf::from(env::var_os("OUT_DIR").unwrap());
    File::create(out.join("memory.x"))
        .unwrap()
        .write_all(include_bytes!("memory.x"))
        .unwrap();
    println!("cargo:rustc-link-search={}", out.display());
}

fn configure_bin_linker_options(linker: Linker) {
    let mut rustc_link_arg_bins_args: Vec<String> = vec!();
    match linker {
        Linker::Gcc => {

            rustc_link_arg_bins_args.push("--verbose".to_string());
        },
        Linker::Llvm => (),
    }
    // this is now supported in both linkers
    rustc_link_arg_bins_args.push("--print-memory-usage".to_string());

    // See https://github.com/rust-embedded/cortex-m-quickstart/pull/95

    rustc_link_arg_bins_args.push("--nmagic".to_string());
    rustc_link_arg_bins_args.push("-Tdefmt.x".to_string());
    rustc_link_arg_bins_args.push("-Tlink.x".to_string());

    //println!("cargo:rustc-link-arg-bins=-Wl,-Map=out.map");
    let mut map_file: PathBuf = PathBuf::from(env::var("OUT_DIR").unwrap());
    map_file.push("out.map");

    rustc_link_arg_bins_args.push(format!("-Map={}", map_file.display()));

    for arg in rustc_link_arg_bins_args {
        println!("cargo:rustc-link-arg-bins={}", format_linker_arg(linker, arg));
    }
}

fn format_linker_arg(linker: Linker, arg: String) -> String {
    let prefix = match linker {
        Linker::Llvm => "",
        Linker::Gcc => "-Wl,"
    };
    format!("{}{}", prefix, arg)
}

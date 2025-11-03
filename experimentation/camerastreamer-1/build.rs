use std::fs;
use std::env;
use std::path::PathBuf;

fn main() {
    // Detect target automatically
    let target = env::var("TARGET").unwrap_or_default();

    if target.contains("gnu") {
        configure_windows_msys2_gnu();
        copy_gnu_dlls();
    } else if target.contains("msvc") {
        configure_windows_msvc();
        copy_msvc_dlls();
    }
}

/// Required when
fn configure_windows_msvc() {
    // MSVC detection
    let vcpkg_root = detect_vcpkg().expect("VCPKG_ROOT not found; install vcpkg or set environment variable");

    let lib_dir = vcpkg_root.join("installed/x64-windows/lib");
    let include_dir = vcpkg_root.join("installed/x64-windows/include");

    println!("cargo:rustc-link-search=native={}", lib_dir.display());
    println!("cargo:rustc-link-lib=static=opencv_world4");
    println!("cargo:include={}", include_dir.display());

    println!("cargo:rerun-if-changed={}", lib_dir.display());
}

fn configure_windows_msys2_gnu() {
    // MSYS2/GNU detection
    let msys2_root = find_msys2_root().unwrap_or_else(|| {
        panic!("Could not find MSYS2 root. Set MSYS2 environment PATH correctly.");
    });

    println!("detected msys2 root: {}", msys2_root.display());

    let lib_dir = msys2_root.join("ucrt64/lib");
    let include_dir = msys2_root.join("ucrt64/include");

    println!("cargo:rustc-link-search=native={}", lib_dir.display());
    println!("cargo:include={}", include_dir.display());

    let libs = [
        "opencv_core",
        "opencv_imgproc",
        "opencv_highgui",
        "opencv_imgcodecs",
    ];
    for lib in &libs {
        println!("cargo:rustc-link-lib=dylib={}", lib);
    }

    println!("cargo:rerun-if-changed={}", lib_dir.display());
}

/// Try to find MSYS2 root from PATH
fn find_msys2_root() -> Option<PathBuf> {
    env::var_os("PATH").and_then(|paths| {
        for path in env::split_paths(&paths) {
            println!("path: {}", path.display());
            if path.ends_with("msys64\\ucrt64\\bin") {
                // Found MSYS2 ucrt64
                if let Some(parent) = path.parent().and_then(|p| p.parent()) {
                    return Some(parent.to_path_buf());
                }
            }
        }
        None
    })
}

/// Detect vcpkg installation automatically if VCPKG_ROOT not set
fn detect_vcpkg() -> Option<PathBuf> {
    if let Ok(root) = env::var("VCPKG_ROOT") {
        Some(PathBuf::from(root))
    } else {
        // Try common install path
        let default = PathBuf::from("D:\\Programs\\vcpkg");
        if default.exists() { Some(default) } else { None }
    }
}

/// Copy MSVC OpenCV DLLs next to binary
fn copy_msvc_dlls() {
    let vcpkg_root = env::var("VCPKG_ROOT").unwrap();
    let out_dir = out_dir();


    let profile = env::var("PROFILE").unwrap(); // "debug" or "release"
    if profile == "release" || profile == "debug" {
        copy_opencv_release_dlls(&vcpkg_root, &out_dir);
//    } else if profile == "debug" {
// disabled, program crashes on startup with the libprotobufd.dll null string error.
//        copy_opencv_debug_dlls(&vcpkg_root, out_dir);
    } else {
        println!("ignoring unknown profile: {}", profile);
    }
}

#[allow(dead_code)]
fn copy_opencv_debug_dlls(vcpkg_root: &String, out_dir: PathBuf) {
    let bin_dir = PathBuf::from(&vcpkg_root).join("installed/x64-windows/debug/bin");
    let debug_dlls = [
        "opencv_world4d",
        "libwebp",
        "jpeg62",
        "libwebpdecoder",
        "libwebpmux",
        "libwebpdemux",
        "libpng16d",
        "liblzma",
        "archive",
        "libcurl-d",
        "tiffd",
        "gif",
        "openjp2",
        "bz2d",
        "zstd",
        "lz4d",
        "leptonica-1.85.0d",
        "libcrypto-3-x64",
        "zlibd1",
        "szip",
        "hdf5_D",
        "abseil_dlld",
        "tesseract55d",
        "libsharpyuv",
        "libprotobufd"
    ];

    copy_dlls(bin_dir.clone(), out_dir.clone(), &debug_dlls.iter().map(|s| s.to_string() + ".dll").collect::<Vec<_>>());
}

fn copy_opencv_release_dlls(vcpkg_root: &String, out_dir: &PathBuf) {
    let bin_dir = PathBuf::from(&vcpkg_root).join("installed/x64-windows/bin");

    let release_dlls = [
        "opencv_world4",
        "libwebp",
        "jpeg62",
        "libwebpdecoder",
        "libwebpmux",
        "libwebpdemux",
        "libpng16",
        "liblzma",
        "archive",
        "libcurl",
        "tiff",
        "gif",
        "openjp2",
        "bz2",
        "zstd",
        "lz4",
        "leptonica-1.85.0",
        "libcrypto-3-x64",
        "zlib1",
        "szip",
        "hdf5",
        "abseil_dll",
        "tesseract55",
        "libsharpyuv",
        "libprotobuf"
    ];

    copy_dlls(bin_dir, out_dir.clone(), &release_dlls.iter().map(|s| s.to_string() + ".dll").collect::<Vec<_>>());
}

fn copy_dlls(bin_dir: PathBuf, out_dir: PathBuf, dlls: &[String]) {
    for dll in dlls {
        let src = bin_dir.join(dll);
        let dest = out_dir.join(dll);

        if src.exists() {
            fs::copy(&src, &dest).expect(&format!("Failed to copy {}", dll));
            println!("cargo:info=Copied {} to {}", dll, dest.display());
        } else {
            println!("cargo:warning=DLL not found: {}", src.display());
        }
    }
}

/// Copy MSYS2 OpenCV DLLs next to binary
fn copy_gnu_dlls() {
    let msys2_root = find_msys2_root().unwrap();
    let bin_dir = msys2_root.join("ucrt64/bin");

    let out_dir = out_dir();

    let dlls = [
        "libopencv_core-411.dll",
        "libopencv_imgproc-411.dll",
        "libopencv_highgui-411.dll",
        "libopencv_imgcodecs-411.dll",
        "libprotobuf.dll",
    ];

    copy_dlls(bin_dir, out_dir, &dlls.iter().map(|s| s.to_string()).collect::<Vec<_>>());
}

/// Determine output directory for build
fn out_dir() -> PathBuf {
    if let Ok(dir) = env::var("CARGO_TARGET_DIR") {
        PathBuf::from(dir)
    } else {
        // default to target/debug or target/release based on PROFILE
        let profile = env::var("PROFILE").unwrap_or("debug".to_string());
        PathBuf::from("target").join(profile)
    }
}

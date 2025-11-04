use std::env;
use std::path::PathBuf;
use std::process::Command;

fn main() {
    //configure_for_windows();
    //configure_for_msys2();


    let opencv_path = vckg_path.join("packages/opencv_x64-windows");

    println!("OpenCV path: {:?}", opencv_path);
    // Set for current process (and anything spawned from it)
    unsafe {
        env::set_var("OpenCV_DIR", &opencv_path);
    }

}

fn configure_for_windows() {
    let mut vckg_path_var = env::var_os("VCPKG_ROOT").unwrap();

    let vckg_path = PathBuf::from(vckg_path_var);

    let opencv_path = vckg_path.join("packages/opencv_x64-windows");

    println!("OpenCV path: {:?}", opencv_path);
    // Set for current process (and anything spawned from it)
    unsafe {
        env::set_var("OpenCV_DIR", &opencv_path);
    }
}

fn configure_for_msys2() {
// Example MSYS2 UCRT64 path
    let msys2_path = PathBuf::from("C:\\msys64\\ucrt64\\bin");

    // Get current PATH
    let mut path_var = env::var_os("PATH").unwrap_or_default();

    // Prepend MSYS2 path
    let mut paths = env::split_paths(&path_var).collect::<Vec<_>>();
    paths.insert(0, msys2_path.clone());
    path_var = env::join_paths(paths).unwrap();

    // Set for current process (and anything spawned from it)
    unsafe {
        env::set_var("PATH", &path_var);
    }

    // Example: check if pkg-config is found
    let status = Command::new("pkg-config")
        .arg("--version")
        .status()
        .expect("failed to run pkg-config");
    if !status.success() {
        panic!("pkg-config not found even after modifying PATH!");
    }

    println!("cargo:rerun-if-changed=build.rs");
}

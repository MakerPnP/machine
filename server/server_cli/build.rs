extern crate rustc_version;

use std::io::{self, Write};
use std::process::exit;
use rustc_version::{version, Version};

fn main() {
    // Check for a minimum version
    let version = version().unwrap();
    let required_version = Version::parse("1.96.0").unwrap();
    writeln!(&mut io::stderr(), "detected rust version: {:?}", version).unwrap();
    if version < required_version {
        writeln!(&mut io::stderr(), "This crate requires rustc >= {:?}, detected: {:?}", required_version, version).unwrap();
        exit(1);
    }
}
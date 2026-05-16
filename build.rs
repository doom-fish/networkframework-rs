// Build the Network.framework C shim and link Network.framework.

use std::env;
use std::path::PathBuf;

fn main() {
    let manifest_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR"));
    let shim_path = manifest_dir.join("src/c-shim/network_shim.c");

    println!("cargo:rerun-if-changed={}", shim_path.display());

    cc::Build::new()
        .file(&shim_path)
        .warnings(true)
        .flag_if_supported("-fblocks")
        .compile("network_shim");

    println!("cargo:rustc-link-lib=framework=Network");
    println!("cargo:rustc-link-lib=framework=Foundation");
    println!("cargo:rustc-link-lib=System");
}

use std::env;
use std::process::Command;

fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=src/c-shim/network_shim.c");
    println!("cargo:rerun-if-changed=src/c-shim/network_shim.h");
    println!("cargo:rerun-if-changed=swift-bridge");
    println!("cargo:rerun-if-env-changed=DOCS_RS");
    println!("cargo:rerun-if-env-changed=DEVELOPER_DIR");
    println!("cargo:rerun-if-env-changed=SDKROOT");
    println!("cargo:rerun-if-env-changed=IPHONEOS_DEPLOYMENT_TARGET");

    if env::var("DOCS_RS").is_ok() {
        return;
    }

    let target_os = env::var("CARGO_CFG_TARGET_OS").unwrap_or_default();
    let target_abi = env::var("CARGO_CFG_TARGET_ABI").unwrap_or_default();
    let target_arch = env::var("CARGO_CFG_TARGET_ARCH").unwrap_or_default();
    let arch = match target_arch.as_str() {
        "x86_64" => "x86_64",
        "aarch64" => "arm64",
        other => panic!(
            "networkframework: unsupported target arch '{other}'. Expected x86_64 or aarch64.",
        ),
    };
    // iOS builds default to the bridge's minimum in swift-bridge/Package.swift.
    let ios_version = env::var("IPHONEOS_DEPLOYMENT_TARGET").unwrap_or_else(|_| "26.0".into());
    let (swift_triple, sdk) = match (target_os.as_str(), target_abi.as_str()) {
        ("ios", "sim") => (
            format!("{arch}-apple-ios{ios_version}-simulator"),
            Some("iphonesimulator"),
        ),
        ("ios", _) => (format!("{arch}-apple-ios{ios_version}"), Some("iphoneos")),
        _ => (format!("{arch}-apple-macosx"), None),
    };

    println!("cargo:rustc-link-lib=framework=Network");
    println!("cargo:rustc-link-lib=framework=Foundation");
    if target_os == "macos" {
        println!("cargo:rustc-link-lib=framework=System");
    }

    let swift_dir = "swift-bridge";
    let out_dir = env::var("OUT_DIR").expect("OUT_DIR");
    let swift_build_dir = format!("{out_dir}/swift-build");

    let mut swift = Command::new("swift");
    swift.args([
        "build",
        "-c",
        "release",
        "--triple",
        &swift_triple,
        "--package-path",
        swift_dir,
        "--scratch-path",
        &swift_build_dir,
    ]);
    if let Some(sdk) = sdk {
        let output = Command::new("xcrun")
            .args(["--sdk", sdk, "--show-sdk-path"])
            .output()
            .expect("Failed to run xcrun");
        assert!(
            output.status.success(),
            "xcrun could not find the {sdk} SDK"
        );
        swift.args(["--sdk", String::from_utf8_lossy(&output.stdout).trim()]);
    }
    let output = swift.output().expect("Failed to build Swift bridge");

    if !output.status.success() {
        eprintln!(
            "Swift build STDOUT:\n{}",
            String::from_utf8_lossy(&output.stdout)
        );
        eprintln!(
            "Swift build STDERR:\n{}",
            String::from_utf8_lossy(&output.stderr)
        );
        panic!(
            "Swift build failed with exit code: {:?}",
            output.status.code()
        );
    }

    println!("cargo:rustc-link-search=native={swift_build_dir}/release");
    println!("cargo:rustc-link-lib=static=NetworkFrameworkBridge");
    println!("cargo:rustc-link-arg=-Wl,-rpath,/usr/lib/swift");

    if target_os != "macos" {
        return;
    }
    if let Ok(output) = Command::new("xcode-select").arg("-p").output() {
        if output.status.success() {
            let xcode_path = String::from_utf8_lossy(&output.stdout).trim().to_string();
            println!(
                "cargo:rustc-link-arg=-Wl,-rpath,{xcode_path}/Toolchains/XcodeDefault.xctoolchain/usr/lib/swift/macosx",
            );
        }
    }
}

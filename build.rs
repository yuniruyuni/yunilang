use std::env;
use std::path::Path;
use std::process::Command;

fn main() {
    // Tell cargo to rerun this build script if it changes
    println!("cargo:rerun-if-changed=build.rs");

    // Print helpful message about LLVM requirement
    println!("cargo:warning=Yuni Language Compiler requires LLVM 18 to be installed.");
    println!("cargo:warning=On macOS: brew install llvm@18");
    println!("cargo:warning=On Ubuntu/Debian: apt-get install llvm-18-dev");
    println!("cargo:warning=Or set LLVM_SYS_180_PREFIX to your LLVM installation path");

    // Check if LLVM is available
    if let Ok(llvm_config) = env::var("LLVM_SYS_180_PREFIX") {
        println!("cargo:rustc-env=LLVM_SYS_180_PREFIX={}", llvm_config);
    } else {
        // Try to find llvm-config in PATH
        let llvm_config_cmd = if cfg!(target_os = "macos") {
            // On macOS, try homebrew paths
            let brew_llvm = "/opt/homebrew/opt/llvm@18/bin/llvm-config";
            let brew_llvm_alt = "/usr/local/opt/llvm@18/bin/llvm-config";

            if Path::new(brew_llvm).exists() {
                brew_llvm
            } else if Path::new(brew_llvm_alt).exists() {
                brew_llvm_alt
            } else {
                "llvm-config-18"
            }
        } else {
            "llvm-config-18"
        };

        if let Ok(output) = Command::new(llvm_config_cmd).arg("--prefix").output() {
            if output.status.success() {
                let prefix = String::from_utf8_lossy(&output.stdout).trim().to_string();
                println!("cargo:rustc-env=LLVM_SYS_180_PREFIX={}", prefix);
            }
        }
    }

    // Link against C++ standard library
    if cfg!(target_os = "macos") {
        println!("cargo:rustc-link-lib=c++");
        // Add Homebrew library path for dependencies like zstd
        println!("cargo:rustc-link-search=/opt/homebrew/lib");
        println!("cargo:rustc-link-search=/usr/local/lib");
    } else if cfg!(target_os = "linux") {
        println!("cargo:rustc-link-lib=stdc++");
    }
}

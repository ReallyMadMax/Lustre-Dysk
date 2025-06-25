//! Simple build.rs that doesn't depend on Args::command()
//! This avoids the clap derive issues entirely

use std::env;

fn main() -> std::io::Result<()> {
    let version = env::var("CARGO_PKG_VERSION").unwrap_or_else(|_| "unknown".to_string());
    
    println!("cargo:rerun-if-changed=src/");
    println!("cargo:rerun-if-changed=Cargo.toml");
    
    eprintln!("Building dysk version: {}", version);
    
    if let Ok(target) = env::var("TARGET") {
        eprintln!("Target architecture: {}", target);
    }
    
    if let Ok(profile) = env::var("PROFILE") {
        eprintln!("Build profile: {}", profile);
    }
    
    eprintln!("✅ Build script completed successfully");
    
    Ok(())
}
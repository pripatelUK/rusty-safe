use std::process::Command;

fn main() {
    // Embed git commit hash
    let output = Command::new("git")
        .args(&["rev-parse", "HEAD"])
        .output()
        .unwrap();
        
    if !output.status.success() {
        panic!("Failed to get git hash: {}", String::from_utf8_lossy(&output.stderr));
    }
    
    let git_hash = String::from_utf8(output.stdout).unwrap();
    println!("cargo:rustc-env=GIT_HASH={}", git_hash.trim());
    
    // Embed build time
    let build_time = chrono::Utc::now().to_rfc3339();
    println!("cargo:rustc-env=BUILD_TIME={}", build_time);
}

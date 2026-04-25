// tools/cargo_version.rs
// vim: set ft=rs
//
use std::fs;

fn main() {
    let cargo_toml = fs::read_to_string("Cargo.toml").expect("Failed to read Cargo.toml");
    let version = extract_version(&cargo_toml).expect("version not found in Cargo.toml");
    println!("{version}");
}

fn extract_version(content: &str) -> Option<String> {
    // Find the [package] section
    let mut in_package = false;

    for line in content.lines() {
        let line = line.trim();

        // Check for section headers
        if line.starts_with('[') && line.ends_with(']') {
            in_package = line == "[package]";
            continue;
        }

        // Only look for version inside [package] section
        if !in_package {
            continue;
        }

        // Match version = "..."
        if line.starts_with("version") {
            if let Some(start) = line.find('"') {
                if let Some(end) = line[(start + 1)..].find('"') {
                    let version = &line[start + 1..start + 1 + end];
                    return Some(version.to_string());
                }
            }
        }
    }

    None
}

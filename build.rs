/*
 *  Copyright (c) 2021-2022 tooboredtocode
 *  All Rights Reserved
 */

use std::env;
use std::process::Command;

fn main() {
    if let Some(version) = get_version() {
        println!("cargo:rustc-env=BUILD_RUSTC_VERSION={}", version);
    }

    if let Some(branch) = get_branch() {
        println!("cargo:rustc-env=BUILD_GIT_BRANCH={}", branch);
    }

    if let Some(revision) = get_revision() {
        println!("cargo:rustc-env=BUILD_GIT_REVISION={}", revision);
    }
}

fn get_version() -> Option<String> {
    let rustc = env::var("RUSTC").unwrap_or("rustc".to_string());

    let version_string = Command::new(rustc)
        .arg("--version")
        .output()
        .ok()
        .and_then(|output| String::from_utf8(output.stdout).ok());

    if let Some(version_string) = version_string {
        let line = version_string.lines().last().unwrap_or(&version_string);
        let mut parts = line.trim().split(" ");

        return parts.nth(1).map(|s| s.to_string());
    }

    None
}

fn get_branch() -> Option<String> {
    Command::new("git")
        .arg("branch")
        .arg("--show-current")
        .output()
        .ok()
        .and_then(|output| String::from_utf8(output.stdout).ok())
}

fn get_revision() -> Option<String> {
    Command::new("git")
        .arg("rev-parse")
        .arg("HEAD")
        .output()
        .ok()
        .and_then(|output| String::from_utf8(output.stdout).ok())
}

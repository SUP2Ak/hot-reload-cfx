// scripts/build.rs
use std::fs;
use std::path::Path;
use std::process::Command;
use std::env;

fn main() {
    println!("What do you want to build ?");
    println!("1. UI (Windows, Linux, MacOS)");
    println!("2. Watcher (Windows, Linux)");
    println!("3. FX Server Resource");
    println!("4. All");
    
    let mut input = String::new();
    std::io::stdin().read_line(&mut input).unwrap();
    
    match input.trim() {
        "1" => build_ui(),
        "2" => build_watcher(),
        "3" => build_fxserver(),
        "4" => {
            build_ui();
            build_watcher();
            build_fxserver();
        }
        _ => println!("Invalid option")
    }
}

fn build_ui() {
    println!("Building UI...");
    let targets = ["x86_64-pc-windows-msvc", "x86_64-unknown-linux-gnu", "x86_64-apple-darwin"];
    
    for target in targets {
        println!("Building for {}", target);
        Command::new("cargo")
            .args(&["build", "--release", "--target", target, "-p", "hot-reload-ui"])
            .status()
            .unwrap();
    }
}

fn build_watcher() {
    println!("Building Watcher...");
    let targets = ["x86_64-pc-windows-msvc", "x86_64-unknown-linux-gnu"];
    
    for target in targets {
        println!("Building for {}", target);
        Command::new("cargo")
            .args(&["build", "--release", "--target", target, "-p", "hot-reload-watcher"])
            .status()
            .unwrap();
    }
}

fn build_fxserver() {
    println!("Building FX Server Resource...");
    Command::new("pnpm")
        .current_dir("resources/hot-reload")
        .arg("build")
        .status()
        .unwrap();
}
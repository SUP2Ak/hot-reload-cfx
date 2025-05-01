use std::process::Command;
use std::io::{self, Write};
use zip;

/*
Target:
- x86_64-pc-windows-msvc
- x86_64-unknown-linux-gnu
- x86_64-apple-darwin (Pas de support pour test & signature pour l'UI)
*/

#[derive(Debug, Clone, PartialEq)]
enum Target {
    Windows,
    Linux,
    MacOS,
}

#[derive(Debug, Clone, PartialEq)]
enum Component {
    UI,
    Watcher,
    FXServer,
}

fn clear_screen() {
    if cfg!(target_os = "windows") {
        std::process::Command::new("cmd")
            .args(["/c", "cls"])
            .spawn()
            .expect("cls command failed to start")
            .wait()
            .expect("failed to wait");
    } else {
        std::process::Command::new("clear")
            .spawn()
            .expect("clear command failed to start")
            .wait()
            .expect("failed to wait");
    }
}

fn pause() {
    print!("\nPress Enter to continue...");
    io::stdout().flush().unwrap();
    io::stdin().read_line(&mut String::new()).unwrap();
}

impl std::fmt::Display for Target {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Target::Windows => write!(f, "Windows"),
            Target::Linux => write!(f, "Linux"),
            Target::MacOS => write!(f, "MacOS"),
        }
    }
}

impl std::fmt::Display for Component {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
                Component::UI => write!(f, "UI"),
                Component::Watcher => write!(f, "Watcher"),
                Component::FXServer => write!(f, "FX Server Resource"),
            }
        }
    }
    
    fn main() {
        let mut selected_components: Vec<Component> = Vec::new();
        let mut selected_targets: Vec<Target> = Vec::new();
        
        loop {
            clear_screen();
            println!("Hot Reload Builder");
            println!("=================\n");
            
            println!("Selected components: {}", if selected_components.is_empty() { 
                "None".to_string() 
            } else { 
                selected_components.iter().map(|c| c.to_string()).collect::<Vec<_>>().join(", ") 
            });
            
            println!("Selected targets: {}\n", if selected_targets.is_empty() { 
                "None".to_string() 
            } else { 
                selected_targets.iter().map(|t| t.to_string()).collect::<Vec<_>>().join(", ") 
            });
            
            println!("Options:");
            println!("1. Select Components");
            println!("2. Select Targets");
            println!("3. Build");
            println!("4. Exit");
            
            print!("\nChoice > ");
            io::stdout().flush().unwrap();
            
            let mut input = String::new();
            io::stdin().read_line(&mut input).unwrap();
        
        match input.trim() {
            "1" => select_components(&mut selected_components),
            "2" => select_targets(&mut selected_targets),
            "3" => {
                if selected_components.is_empty() || selected_targets.is_empty() {
                    println!("\nPlease select at least one component and target!");
                    pause();
                    continue;
                }
                build(selected_components.clone(), selected_targets.clone());
                break;
            },
            "4" => break,
            _ => {
                println!("\nInvalid option!");
                pause();
            }
        }
    }
}

fn select_components(selected: &mut Vec<Component>) {
    loop {
        clear_screen();
        println!("Select Components");
        println!("================\n");
        
        println!("Current selection: {}\n", if selected.is_empty() { 
            "None".to_string() 
        } else { 
            selected.iter().map(|c| c.to_string()).collect::<Vec<_>>().join(", ") 
        });

        println!("Options:");
        println!("1. UI");
        println!("2. Watcher");
        println!("3. FX Server Resource");
        println!("4. All");
        println!("5. Clear selection");
        println!("6. Back");
        
        print!("\nChoice > ");
        io::stdout().flush().unwrap();
        
        let mut input = String::new();
        io::stdin().read_line(&mut input).unwrap();
        
        match input.trim() {
            "1" => if !selected.contains(&Component::UI) { selected.push(Component::UI) },
            "2" => if !selected.contains(&Component::Watcher) { selected.push(Component::Watcher) },
            "3" => if !selected.contains(&Component::FXServer) { selected.push(Component::FXServer) },
            "4" => {
                selected.clear();
                selected.extend_from_slice(&[Component::UI, Component::Watcher, Component::FXServer]);
            },
            "5" => selected.clear(),
            "6" => break,
            _ => {
                println!("\nInvalid option!");
                pause();
            }
        }
    }
}

fn select_targets(selected: &mut Vec<Target>) {
    loop {
        clear_screen();
        println!("Select Targets");
        println!("=============\n");
        
        println!("Current selection: {}\n", if selected.is_empty() { 
            "None".to_string() 
        } else { 
            selected.iter().map(|t| t.to_string()).collect::<Vec<_>>().join(", ") 
        });

        println!("Options:");
        println!("1. Windows");
        println!("2. Linux");
        println!("3. MacOS");
        println!("4. All");
        println!("5. Clear selection");
        println!("6. Back");
        
        print!("\nChoice > ");
        io::stdout().flush().unwrap();
        
        let mut input = String::new();
        io::stdin().read_line(&mut input).unwrap();
        
        match input.trim() {
            "1" => if !selected.contains(&Target::Windows) { selected.push(Target::Windows) },
            "2" => if !selected.contains(&Target::Linux) { selected.push(Target::Linux) },
            "3" => if !selected.contains(&Target::MacOS) { selected.push(Target::MacOS) },
            "4" => {
                selected.clear();
                selected.extend_from_slice(&[Target::Windows, Target::Linux, Target::MacOS]);
            },
            "5" => selected.clear(),
            "6" => break,
            _ => {
                println!("\nInvalid option!");
                pause();
            }
        }
    }
}

impl Target {
    fn target_triple(&self) -> &'static str {
        match self {
            Target::Windows => "x86_64-pc-windows-msvc",
            Target::Linux => "x86_64-unknown-linux-gnu",
            Target::MacOS => "x86_64-apple-darwin",
        }
    }
}

fn build(components: Vec<Component>, targets: Vec<Target>) {
    clear_screen();
    println!("Building...\n");

    let root_path = std::env::current_dir().unwrap();
    println!("📂 Building from path: {}", root_path.display());
    
    let dockerfile_path = root_path.join("Dockerfile.build");
    if !dockerfile_path.exists() {
        println!("❌ Dockerfile.build not found at: {}", dockerfile_path.display());
        pause();
        return;
    }
    
    let root_path_clone = root_path.clone();
    let mut build_completed = false;

    for component in components {
        for target in &targets {
            match (&component, target) {
                (Component::UI, Target::MacOS) => {
                    println!("⚠️  UI is not supported on MacOS yet, skipping...");
                    continue;
                },
                (Component::Watcher, Target::MacOS) => {
                    println!("⚠️  Watcher is not supported on MacOS, skipping...");
                    continue;
                },
                (Component::UI, Target::Linux) => {
                    println!("⚠️  UI is not supported on Linux yet, skipping...");
                    continue;
                },
                (Component::UI, Target::Windows) => {
                    println!("🔨 Building UI for Windows...");
                    let status = Command::new("cargo")
                        .args(&["build", "--release", "--target", target.target_triple(), "-p", "hot-reload-ui"])
                        .current_dir(&root_path_clone)
                        .status()
                        .unwrap();

                    if status.success() {
                        let source = root_path_clone
                            .join("target")
                            .join(target.target_triple())
                            .join("release")
                            .join("hot-reload-ui.exe");
                        
                        let dest_dir = root_path_clone
                            .join("target")
                            .join("windows");
                        
                        std::fs::create_dir_all(&dest_dir).unwrap();
                        let dest = dest_dir.join("hot-reload-ui.exe");
                        
                        if let Err(e) = std::fs::copy(&source, &dest) {
                            println!("❌ Failed to copy UI executable: {}", e);
                        } else {
                            let _ = std::fs::remove_dir_all(root_path_clone
                                .join("target")
                                .join(target.target_triple()));
                            println!("✅ UI built successfully! Location: {}", dest.display());
                            build_completed = true;
                        }
                    }
                },
                (Component::Watcher, Target::Linux) => {
                    println!("🔨 Building Watcher for Linux using Docker...");
                    let output = Command::new("docker")
                        .args(&[
                            "build",
                            "-f",
                            dockerfile_path.to_str().unwrap(),
                            ".",
                            "--build-arg",
                            "COMPONENT=hot-reload-watcher",
                            "-t",
                            "hot-reload-builder"
                        ])
                        .current_dir(&root_path_clone)
                        .output()
                        .unwrap();

                    if !output.status.success() {
                        println!("❌ Docker build failed: {}", String::from_utf8_lossy(&output.stderr));
                        continue;
                    }

                    let container_output = Command::new("docker")
                        .args(&[
                            "create",
                            "hot-reload-builder"
                        ])
                        .output()
                        .unwrap();

                    let container_id = String::from_utf8(container_output.stdout).unwrap();
                    let container_id = container_id.trim();
                    println!("📦 Container ID: {}", container_id);

                    let output_dir = root_path_clone.join("target").join("linux");
                    std::fs::create_dir_all(&output_dir).unwrap();
                    println!("📂 Output directory: {}", output_dir.display());
                    
                    let cp_status = Command::new("docker")
                        .args(&[
                            "cp",
                            &format!("{}:/app/hot-reload-watcher", container_id),
                            output_dir.join("hot-reload-watcher").to_str().unwrap()
                        ])
                        .status()
                        .unwrap();

                    if !cp_status.success() {
                        println!("❌ Failed to copy binary from container!");
                        continue;
                    }

                    let _ = Command::new("docker")
                        .args(&["rm", container_id])
                        .status();

                    println!("✅ Watcher built successfully! Location: {}", output_dir.display());
                    build_completed = true;
                },
                (Component::Watcher, Target::Windows) => {
                    println!("🔨 Building Watcher for Windows...");
                    let status = Command::new("cargo")
                        .args(&["build", "--release", "--target", target.target_triple(), "-p", "hot-reload-watcher"])
                        .current_dir(&root_path_clone)
                        .status()
                        .unwrap();

                    if status.success() {
                        let source = root_path_clone
                            .join("target")
                            .join(target.target_triple())
                            .join("release")
                            .join("hot-reload-watcher.exe");
                        
                        let dest_dir = root_path_clone
                            .join("target")
                            .join("windows");
                        
                        std::fs::create_dir_all(&dest_dir).unwrap();
                        let dest = dest_dir.join("hot-reload-watcher.exe");
                        
                        if let Err(e) = std::fs::copy(&source, &dest) {
                            println!("❌ Failed to copy Watcher executable: {}", e);
                        } else {
                            let _ = std::fs::remove_dir_all(root_path_clone
                                .join("target")
                                .join(target.target_triple()));
                            println!("✅ Watcher built successfully! Location: {}", dest.display());
                            build_completed = true;
                        }
                    }
                },
                (Component::FXServer, _) => {
                    println!("🔨 Building FX Server Resource...");
                    let resources_path = root_path_clone.join("resources").join("hot-reload");
                    if resources_path.exists() {
                        let output_dir = root_path_clone.join("target");
                        std::fs::create_dir_all(&output_dir).unwrap();
                        
                        let zip_path = output_dir.join("hot-reload-fxserver.zip");
                        let file = std::fs::File::create(&zip_path).unwrap();
                        let mut zip = zip::ZipWriter::new(file);
                        let options = zip::write::FileOptions::default()
                            .compression_method(zip::CompressionMethod::Stored);
                        
                        fn add_dir_to_zip(
                            zip: &mut zip::ZipWriter<std::fs::File>,
                            dir_path: &std::path::Path,
                            base_path: &std::path::Path,
                            options: zip::write::FileOptions
                        ) -> std::io::Result<()> {
                            for entry in std::fs::read_dir(dir_path)? {
                                let entry = entry?;
                                let path = entry.path();
                                let name = path.strip_prefix(base_path).unwrap();
                                
                                if path.to_str().unwrap().contains("node_modules") ||
                                   path.to_str().unwrap().contains("dist") ||
                                   path.to_str().unwrap().contains(".git") ||
                                   path.to_str().unwrap().contains("pnpm-lock.yaml") ||
                                   path.to_str().unwrap().contains(".gitignore") {
                                    continue;
                                }
                                
                                if path.is_file() {
                                    zip.start_file(format!("hot-reload/{}", name.to_str().unwrap().replace("\\", "/")), options)?;
                                    let mut f = std::fs::File::open(path)?;
                                    std::io::copy(&mut f, zip)?;
                                } else if path.is_dir() {
                                    zip.add_directory(format!("hot-reload/{}/", name.to_str().unwrap().replace("\\", "/")), options)?;
                                    add_dir_to_zip(zip, &path, base_path, options)?;
                                }
                            }
                            Ok(())
                        }
                        
                        if let Err(e) = add_dir_to_zip(&mut zip, &resources_path, &resources_path, options) {
                            println!("❌ Failed to create zip: {}", e);
                        } else {
                            if let Err(e) = zip.finish() {
                                println!("❌ Failed to finalize zip: {}", e);
                            } else {
                                println!("✅ FX Server Resource packaged successfully!");
                                println!("📦 Location: {}", zip_path.display());
                                build_completed = true;
                            }
                        }
                    } else {
                        println!("⚠️  FX Server Resource path not found!");
                    }
                    break;
                },
            }
        }
    }
    
    if build_completed {
        println!("\n✅ Build completed!");
    } else {
        println!("\n❌ Build failed!");
    }
    pause();
}
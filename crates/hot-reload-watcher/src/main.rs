use hot_reload_watcher::ResourceWatcher;
use hot_reload_watcher::WatcherConfig;
use std::io::Write;
use std::path::Path;
use std::sync::Arc;
use tokio::io::{self, AsyncBufReadExt};
use tracing::error;
use tracing_appender;

#[tokio::main]
async fn main() {
    std::fs::create_dir_all("logs").expect("Failed to create logs directory");
    
    let file_appender = tracing_appender::rolling::never(
        "logs",
        "watcher.log"
    );
    let (non_blocking, _guard) = tracing_appender::non_blocking(file_appender);

    tracing_subscriber::fmt()
        .with_writer(non_blocking)
        .with_ansi(true)
        .with_target(false)
        .with_thread_ids(false)
        .with_file(false)
        .with_line_number(false)
        .event_format(
            tracing_subscriber::fmt::format()
                .with_level(true)
                .with_target(false)
                .with_thread_ids(false)
                .with_file(false)
                .with_line_number(false)
                .with_ansi(true)
        )
        .init();

    if !Path::new("resources").is_dir() {
        error!("The 'resources' directory does not exist in the current directory!");
        std::process::exit(1);
    }

    let config = WatcherConfig::load_or_create();
    let watcher_handle = tokio::spawn(async move {
        let watcher = Arc::new(ResourceWatcher::new(config));
        if let Err(e) = watcher.run().await {
            error!("Error running watcher: {}", e);
            std::process::exit(1);
        }
    });

    if cfg!(target_os = "windows") {
        std::process::Command::new("cmd")
            .args(["/c", "cls"])
            .spawn()
            .expect("cls command failed to start")
            .wait()
            .expect("failed to wait");
    } else if cfg!(target_os = "linux") {
        std::process::Command::new("clear")
            .spawn()
            .expect("clear command failed to start")
            .wait()
            .expect("failed to wait");
    } else {
        // TODO: Add support for other OS
        println!("🚫 Unsupported OS");
        std::process::exit(1);
    }

    let mut reader = io::BufReader::new(io::stdin()).lines();
    println!("🚀 Hot Reload Watcher v{}", env!("CARGO_PKG_VERSION"));
    println!("📝 Type 'help' for available commands.\n");
    print!("> ");
    std::io::stdout().flush().expect("Failed to flush stdout");
    
    while let Ok(Some(line)) = reader.next_line().await {
        match line.trim().to_lowercase().as_str() {
            "help" => {
                println!("\n📋 Available commands:");
                println!("- help        : Show this help message");
                println!("- clear | cls : Clear the terminal");
                println!("- exit | quit : Stop the watcher and exit");
                println!("- status      : Show current watcher status");
                println!("- version     : Show current version");
                println!("- logs        : Open logs file");
                println!("- console     : Open watcher console");
            },
            "cls" | "clear" => {
                if cfg!(target_os = "windows") {
                    std::process::Command::new("cmd")
                        .args(["/c", "cls"])
                        .spawn()
                        .expect("cls command failed to start")
                        .wait()
                        .expect("failed to wait");
                } else if cfg!(target_os = "linux") {
                    std::process::Command::new("clear")
                        .spawn()
                        .expect("clear command failed to start")
                        .wait()
                        .expect("failed to wait");
                } else {
                    println!("🚫 Unsupported OS");
                    std::process::exit(1);
                }
            },
            "exit" | "quit" => {
                println!("👋 Stopping watcher...");
                break;
            },
            "status" => {
                println!("✨ Watcher is running");
            },
            "version" => {
                println!("🔍 Version: {}", env!("CARGO_PKG_VERSION"));
            },
            "logs" => {
                if cfg!(target_os = "windows") {
                    std::process::Command::new("explorer")
                        .args(["logs\\watcher.log"])
                        .spawn()
                        .expect("failed to open logs");
                } else if cfg!(target_os = "linux") {
                    std::process::Command::new("xdg-open")
                        .arg("logs/watcher.log")
                        .spawn()
                        .expect("failed to open logs");
                } else {
                    println!("🚫 Unsupported OS");
                    std::process::exit(1);
                }
                println!("📝 Logs file opened");
            },
            "console" => {
                if cfg!(target_os = "windows") {
                    let path = std::env::current_dir().unwrap();
                    let script_path = path.join("logs\\watch_script.ps1");
                    let log_path = path.join("logs\\watcher.log");
                    
                    let ps_script = format!(r#"
                        $OutputEncoding = [Console]::OutputEncoding = [Text.Encoding]::UTF8
                        Write-Host "Watcher Console - Press Ctrl+C to close`n"
                        $lastLine = ""
                        while($true) {{
                            if(Test-Path "{}") {{
                                $currentLine = Get-Content "{}" -Tail 1 -Encoding utf8
                                if($currentLine -ne $lastLine) {{
                                    Write-Host $currentLine
                                    $lastLine = $currentLine
                                }}
                            }}
                            Start-Sleep -Milliseconds 100
                        }}
                    "#, log_path.display(), log_path.display());

                    std::fs::write(script_path.clone(), ps_script).expect("Failed to create PowerShell script");
                    std::process::Command::new("cmd")
                        .args(["/c", "start", "powershell", "-NoExit", "-ExecutionPolicy", "Bypass", "-File", script_path.to_str().unwrap()])
                        .spawn()
                        .expect("failed to open console");
                } else if cfg!(target_os = "linux") {
                    // Vérifier si screen est installé
                    let has_screen = std::process::Command::new("which")
                        .arg("screen")
                        .output()
                        .map(|output| output.status.success())
                        .unwrap_or(false);

                    if !has_screen {
                        println!("📥 Installing screen...");
                        if let Err(e) = std::process::Command::new("sudo")
                            .args(["apt-get", "update"])
                            .status() {
                            println!("❌ Failed to update apt: {}", e);
                            return;
                        } else {
                            println!("✅ Updated apt");
                        }
                        
                        if let Err(e) = std::process::Command::new("sudo")
                            .args(["apt-get", "install", "-y", "screen"])
                            .status() {
                            println!("❌ Failed to install screen: {}", e);
                            return;
                        } else {
                            println!("✅ Installed screen");
                        }
                    }

                    let screen_name = "watcher-console";
                    let _ = std::process::Command::new("screen")
                        .args(["-X", "-S", screen_name, "quit"])
                        .output();

                    match std::process::Command::new("screen")
                        .args([
                            "-dmS", 
                            screen_name,
                            "bash",
                            "-c",
                            &format!("cd '{}' && echo 'Watcher Console - Press Ctrl+A then D to detach, Ctrl+C to close\n' && tail -f logs/watcher.log", 
                                std::env::current_dir().unwrap().display())
                        ])
                        .spawn() {
                            Ok(_) => {
                                // Attacher à la session
                                if let Err(e) = std::process::Command::new("screen")
                                    .args(["-r", screen_name])
                                    .status() {
                                    println!("❌ Failed to attach to screen: {}", e);
                                    return;
                                } else {
                                    println!("✅ Attached to screen");
                                }
                                println!("📺 Console opened in screen session");
                                println!("💡 To reattach later: screen -r {}", screen_name);
                            },
                            Err(e) => {
                                println!("❌ Failed to create screen session: {}", e);
                                println!("💡 Try manually: tail -f logs/watcher.log");
                            }
                        }
                }
            },
            "" => {},
            cmd => {
                println!("❌ Unknown command: '{}'. Type 'help' for available commands.", cmd);
            }
        }
        print!("> ");
        std::io::stdout().flush().expect("Failed to flush stdout");
    }

    watcher_handle.abort();
    println!("👋 Watcher stopped");
}
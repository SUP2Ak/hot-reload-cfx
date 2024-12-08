use hot_reload_watcher::WatcherConfig;
use tracing::error;
use tokio::io::{self, AsyncBufReadExt};
use tracing_appender;
use std::io::Write;
use std::path::Path;

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
        if let Err(e) = hot_reload_watcher::run(config).await {
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
    } else {
        print!("\x1B[2J\x1B[1;1H");
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
                println!("- help     : Show this help message");
                println!("- clear    : Clear the terminal");
                println!("- exit     : Stop the watcher and exit");
                println!("- status   : Show current watcher status");
                println!("- version  : Show current version");
                println!("- logs     : Open logs file");
                println!("- console  : Open watcher console");
            },
            "cls" | "clear" => {
                if cfg!(target_os = "windows") {
                    std::process::Command::new("cmd")
                        .args(["/c", "cls"])
                        .spawn()
                        .expect("cls command failed to start")
                        .wait()
                        .expect("failed to wait");
                } else {
                    print!("\x1B[2J\x1B[1;1H");
                }
            },
            "exit" => {
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
                } else {
                    std::process::Command::new("xdg-open")
                        .arg("logs/watcher.log")
                        .spawn()
                        .expect("failed to open logs");
                }
                println!("📝 Logs file opened");
            },
            "console" => {
                if cfg!(target_os = "windows") {
                    let ps_script = r#"
                        $OutputEncoding = [Console]::OutputEncoding = [Text.Encoding]::UTF8
                        Write-Host "Watcher Console - Press Ctrl+C to close`n"
                        Get-Content "logs\watcher.log" -Wait -Encoding utf8 | ForEach-Object {
                            Write-Host $_
                        }
                    "#;
                    let script_path = "logs\\watch_script.ps1";
                    std::fs::write(script_path, ps_script).expect("Failed to create PowerShell script");

                    std::process::Command::new("cmd")
                        .args(["/c", "start", "powershell", "-NoExit", "-ExecutionPolicy", "Bypass", "-File", script_path])
                        .spawn()
                        .expect("failed to open console");
                } else {
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
                        }
                        
                        if let Err(e) = std::process::Command::new("sudo")
                            .args(["apt-get", "install", "-y", "screen"])
                            .status() {
                            println!("❌ Failed to install screen: {}", e);
                            return;
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
#![cfg_attr(target_os = "windows", windows_subsystem = "windows")]

mod runtime;
mod shell;
mod update;

use clap::{Parser, Subcommand};
use tracing_subscriber::EnvFilter;

#[cfg(target_os = "windows")]
fn attach_parent_console_if_cli() {
    // Only attach to an existing parent console when CLI arguments are provided
    // (e.g. `ace info`, `ace query`, `ace --help`), so launching the GUI never
    // spawns or attaches a terminal window.
    let mut args = std::env::args_os().skip(1);
    if let Some(first) = args.next() {
        if first != "gui" {
            const ATTACH_PARENT_PROCESS: u32 = u32::MAX;
            extern "system" {
                fn AttachConsole(dw_process_id: u32) -> i32;
            }
            unsafe {
                let _ = AttachConsole(ATTACH_PARENT_PROCESS);
            }
        }
    }
}

#[derive(Parser, Debug)]
#[command(name = "ace", version, about = "Ace — cross-platform Sequel Ace remake (GPUI)")]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// Launch the GPUI desktop app (default)
    Gui,
    /// Open an interactive CLI shell against a connection string
    Connect {
        url: String,
    },
    Query {
        url: String,
        sql: String,
    },
    Export {
        url: String,
        sql: String,
        #[arg(short, long)]
        output: String,
        #[arg(long, default_value = "csv")]
        format: String,
    },
    Mcp {
        #[arg(long, default_value = "127.0.0.1")]
        host: String,
        #[arg(long, default_value_t = 3847)]
        port: u16,
        #[arg(long, default_value_t = true)]
        read_only: bool,
    },
    Info,
    UpdateCheck,
}

fn main() {
    #[cfg(target_os = "windows")]
    attach_parent_console_if_cli();

    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env().add_directive("ace=info".parse().unwrap()))
        .init();

    let cli = Cli::parse();
    match cli.command.unwrap_or(Commands::Gui) {
        Commands::Gui => {
            ace_ui::run_gui();
        }
        Commands::Info => {
            println!("Ace {}", env!("CARGO_PKG_VERSION"));
            println!("UI: GPUI Kit 0.10.0 (latest)");
            println!("Drivers: mysql, mariadb, postgres, sqlite");
            println!("Default: `ace` or `ace gui` opens the desktop app.");
        }
        Commands::Connect { url } => {
            let rt = tokio::runtime::Runtime::new().unwrap();
            rt.block_on(shell::run_shell(&url)).unwrap();
        }
        Commands::Query { url, sql } => {
            let rt = tokio::runtime::Runtime::new().unwrap();
            rt.block_on(runtime::run_query(&url, &sql)).unwrap();
        }
        Commands::Export {
            url,
            sql,
            output,
            format,
        } => {
            let rt = tokio::runtime::Runtime::new().unwrap();
            rt.block_on(runtime::run_export(&url, &sql, &output, &format))
                .unwrap();
        }
        Commands::Mcp {
            host,
            port,
            read_only,
        } => {
            let rt = tokio::runtime::Runtime::new().unwrap();
            rt.block_on(runtime::run_mcp(&host, port, read_only)).unwrap();
        }
        Commands::UpdateCheck => {
            let rt = tokio::runtime::Runtime::new().unwrap();
            match rt.block_on(update::check_latest_version()) {
                Ok(Some(latest)) => {
                    let current = update::current_version();
                    if latest != current {
                        println!("update available: {latest} (current {current})");
                    } else {
                        println!("up to date ({current})");
                    }
                }
                Ok(None) => println!("no release info"),
                Err(e) => println!("update check failed: {e}"),
            }
        }
    }
}
#![forbid(unsafe_code)]

use std::path::PathBuf;
use std::process::ExitCode;

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let mut socket: Option<PathBuf> = None;
    let mut index = 0;
    while index < args.len() {
        match args[index].as_str() {
            "--socket" => {
                let Some(value) = args.get(index + 1) else {
                    eprintln!("--socket requires a path");
                    return ExitCode::FAILURE;
                };
                socket = Some(PathBuf::from(value));
                index += 1;
            }
            "--version" => {
                println!("tech2d {}", env!("CARGO_PKG_VERSION"));
                return ExitCode::SUCCESS;
            }
            "--help" | "-h" => {
                help();
                return ExitCode::SUCCESS;
            }
            other => {
                eprintln!("unknown argument: {other}");
                help();
                return ExitCode::FAILURE;
            }
        }
        index += 1;
    }
    let socket = socket.unwrap_or_else(tech2d::default_socket_path);
    println!(
        "tech2d {} listening on {}",
        env!("CARGO_PKG_VERSION"),
        socket.display()
    );
    match tech2d::serve(&socket) {
        Ok(()) => ExitCode::SUCCESS,
        Err(err) => {
            eprintln!("error: {err}");
            ExitCode::FAILURE
        }
    }
}

fn help() {
    println!("tech2d {}", env!("CARGO_PKG_VERSION"));
    println!("usage: tech2d [--socket PATH] [--version]");
    println!("  --socket PATH   Unix socket to bind");
    println!("  TECH2D_SOCKET   environment override for the socket path");
}

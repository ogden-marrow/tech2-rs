#![forbid(unsafe_code)]

use std::env;
use std::fs;
use std::path::Path;
use std::process::ExitCode;

use tech2_core::inspect_image;
use tech2_tis::DataStore;

fn main() -> ExitCode {
    let args: Vec<String> = env::args().skip(1).collect();
    match args.first().map(String::as_str) {
        Some("inspect") => inspect(args.get(1).map(String::as_str)),
        Some("ports") => ports(),
        Some("mdb") => mdb(args.get(1).map(String::as_str)),
        Some("help" | "--help" | "-h") | None => {
            help();
            ExitCode::SUCCESS
        }
        Some(other) => {
            eprintln!("unknown command: {other}");
            help();
            ExitCode::FAILURE
        }
    }
}

fn inspect(path: Option<&str>) -> ExitCode {
    let Some(path) = path else {
        eprintln!("usage: tech2ctl inspect <image.bin>");
        return ExitCode::FAILURE;
    };
    match inspect_image(Path::new(path), "9-5") {
        Ok(info) => {
            println!("file:      {}", info.path.display());
            println!("size:      {:.0} MB", info.size_mb());
            println!("version:   {}", info.version_label);
            println!(
                "header:    {}",
                if info.magic_ok {
                    "ok"
                } else {
                    "missing T2 header"
                }
            );
            println!("coverage:  {}", info.coverage.unwrap_or("unknown"));
            for warning in &info.warnings {
                println!("warning:   {warning}");
            }
            if info.valid() {
                ExitCode::SUCCESS
            } else {
                ExitCode::FAILURE
            }
        }
        Err(err) => {
            eprintln!("error: {err}");
            ExitCode::FAILURE
        }
    }
}

fn ports() -> ExitCode {
    let mut found = Vec::new();
    match fs::read_dir("/dev") {
        Ok(entries) => {
            for entry in entries.flatten() {
                let name = entry.file_name().to_string_lossy().to_string();
                let interesting = name.starts_with("ttyUSB")
                    || name.starts_with("ttyACM")
                    || matches!(name.as_str(), "ttyS0" | "ttyS1" | "ttyS2" | "ttyS3");
                if interesting {
                    found.push(entry.path());
                }
            }
        }
        Err(err) => {
            eprintln!("cannot read /dev: {err}");
            return ExitCode::FAILURE;
        }
    }
    found.sort();
    if found.is_empty() {
        println!("No serial ports found. Plug in the Tech 2 adapter and try again.");
    } else {
        for path in found {
            println!("{}", path.display());
        }
    }
    ExitCode::SUCCESS
}

fn mdb(root: Option<&str>) -> ExitCode {
    let root = root.unwrap_or(".");
    match DataStore::open(root) {
        Ok(store) => {
            println!("cosids:    {}", store.root().display());
            println!("mdb:       {}", store.mdb_path().display());
            println!(
                "mdbtools:  {}",
                if DataStore::mdb_tools_available() {
                    "available"
                } else {
                    "not installed"
                }
            );
            for dir in store.superbase_dirs() {
                println!("superbase: {}", dir.display());
            }
            ExitCode::SUCCESS
        }
        Err(err) => {
            eprintln!("error: {err}");
            ExitCode::FAILURE
        }
    }
}

fn help() {
    println!("tech2ctl {}", env!("CARGO_PKG_VERSION"));
    println!("usage:");
    println!("  tech2ctl inspect <image.bin>   validate a Tech 2 card image");
    println!("  tech2ctl ports                 list candidate serial devices");
    println!("  tech2ctl mdb [cosids-dir]      inspect a TIS2000 data directory");
    println!("  tech2ctl help");
}

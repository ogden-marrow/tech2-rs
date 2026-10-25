#![forbid(unsafe_code)]

use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use serde_json::{json, Value};
use tech2_core::inspect_image;
use tech2_tis::mdb::{sanitize_table_name, MdbTools};
use tech2_tis::superbase;
use tech2_tis::{DataStore, SUPERBASE_DIRS};

fn main() -> ExitCode {
    let args: Vec<String> = env::args().skip(1).collect();
    match args.first().map(String::as_str) {
        Some("inspect") => inspect(args.get(1).map(String::as_str)),
        Some("ports") => ports(),
        Some("mdb") => mdb(args.get(1).map(String::as_str)),
        Some("tis") => tis(&args[1..]),
        Some("rpc") => rpc(&args[1..]),
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
    let ports = tech2_core::ports::list_ports();
    if ports.is_empty() {
        println!("No serial ports found. Plug in the Tech 2 adapter and try again.");
    } else {
        for port in ports {
            println!("{}\t{}", port.device.display(), port.kind);
        }
    }
    ExitCode::SUCCESS
}

fn mdb(root: Option<&str>) -> ExitCode {
    let root = root.unwrap_or(".");
    match DataStore::open(root) {
        Ok(store) => {
            println!("cosids:    {}", store.root().display());
            let db = store.mdb_path();
            println!("mdb:       {}", db.display());
            let tools = MdbTools::from_env();
            if tools.available() {
                match tools.tables(&db) {
                    Ok(tables) => {
                        println!("tables:    {}", tables.len());
                        for table in tables.iter().take(20) {
                            println!("  {table}");
                        }
                    }
                    Err(err) => println!("tables:    error: {err}"),
                }
            } else {
                println!("mdbtools:  not installed (set TECH2_MDBTOOLS or install mdbtools)");
            }
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

fn tis(args: &[String]) -> ExitCode {
    match args.first().map(String::as_str) {
        Some("export") => tis_export(&args[1..]),
        Some("probe") => tis_probe(&args[1..]),
        _ => {
            eprintln!("usage: tech2ctl tis <export|probe> [options]");
            ExitCode::FAILURE
        }
    }
}

fn tis_export(args: &[String]) -> ExitCode {
    let Some(out) = arg_value(args, "--out") else {
        eprintln!("usage: tech2ctl tis export --cosids <dir> --out <dir>");
        return ExitCode::FAILURE;
    };
    let cosids = arg_value(args, "--cosids").unwrap_or(".");
    let store = match DataStore::open(cosids) {
        Ok(store) => store,
        Err(err) => {
            eprintln!("error: {err}");
            return ExitCode::FAILURE;
        }
    };
    let out = PathBuf::from(out);
    let out_abs = match env::current_dir() {
        Ok(cwd) => cwd.join(&out),
        Err(err) => {
            eprintln!("error: {err}");
            return ExitCode::FAILURE;
        }
    };
    if out_abs.starts_with(store.root()) {
        eprintln!("refusing to write inside the source data directory");
        return ExitCode::FAILURE;
    }
    let tools = MdbTools::from_env();
    if !tools.available() {
        eprintln!(
            "mdbtools not found. Install the mdbtools package or set TECH2_MDBTOOLS to its bin directory."
        );
        return ExitCode::FAILURE;
    }
    let db = store.mdb_path();
    let tables = match tools.tables(&db) {
        Ok(tables) => tables,
        Err(err) => {
            eprintln!("error: {err}");
            return ExitCode::FAILURE;
        }
    };
    if let Err(err) = fs::create_dir_all(&out) {
        eprintln!("cannot create {}: {err}", out.display());
        return ExitCode::FAILURE;
    }
    let mut exported = 0usize;
    for table in &tables {
        match tools.export_table(&db, table) {
            Ok(csv) => {
                let path = out.join(format!("{}.csv", sanitize_table_name(table)));
                if let Err(err) = fs::write(&path, csv) {
                    eprintln!("cannot write {}: {err}", path.display());
                    return ExitCode::FAILURE;
                }
                exported += 1;
            }
            Err(err) => eprintln!("skipping {table}: {err}"),
        }
    }
    let manifest = format!(
        "source: {}\ntables: {}\nexported: {}\n",
        db.display(),
        tables.len(),
        exported
    );
    if let Err(err) = fs::write(out.join("manifest.txt"), manifest) {
        eprintln!("cannot write manifest: {err}");
        return ExitCode::FAILURE;
    }
    println!(
        "exported {exported} of {} tables to {}",
        tables.len(),
        out.display()
    );
    if exported == tables.len() {
        ExitCode::SUCCESS
    } else {
        ExitCode::FAILURE
    }
}

fn tis_probe(args: &[String]) -> ExitCode {
    let cosids = arg_value(args, "--cosids").unwrap_or(".");
    let store = match DataStore::open(cosids) {
        Ok(store) => store,
        Err(err) => {
            eprintln!("error: {err}");
            return ExitCode::FAILURE;
        }
    };
    for name in SUPERBASE_DIRS {
        let file = store.root().join(name).join("rfile000");
        if !file.is_file() {
            println!("{name}: missing");
            continue;
        }
        match superbase::probe(&file) {
            Ok(probe) => println!(
                "{name}: size={} magic_ok={} name={}",
                probe.size,
                probe.magic_ok,
                probe.name.as_deref().unwrap_or("unknown")
            ),
            Err(err) => println!("{name}: error: {err}"),
        }
    }
    ExitCode::SUCCESS
}

fn rpc(args: &[String]) -> ExitCode {
    let Some(method) = args
        .first()
        .filter(|arg| !arg.starts_with("--"))
        .map(String::as_str)
    else {
        eprintln!("usage: tech2ctl rpc <method> [json-params] [--socket PATH]");
        return ExitCode::FAILURE;
    };
    let params = match args.get(1).filter(|arg| !arg.starts_with("--")) {
        Some(raw) => match serde_json::from_str::<Value>(raw) {
            Ok(value) => value,
            Err(err) => {
                eprintln!("invalid JSON params: {err}");
                return ExitCode::FAILURE;
            }
        },
        None => json!({}),
    };
    let socket = arg_value(args, "--socket")
        .map(PathBuf::from)
        .unwrap_or_else(tech2d::default_socket_path);
    match tech2d::rpc_call(&socket, method, params) {
        Ok(result) => {
            let text = serde_json::to_string_pretty(&result).unwrap_or_else(|_| result.to_string());
            println!("{text}");
            ExitCode::SUCCESS
        }
        Err(err) => {
            eprintln!("error: {err}");
            ExitCode::FAILURE
        }
    }
}

fn arg_value<'a>(args: &'a [String], flag: &str) -> Option<&'a str> {
    args.iter()
        .position(|arg| arg == flag)
        .and_then(|index| args.get(index + 1))
        .map(String::as_str)
}

fn help() {
    println!("tech2ctl {}", env!("CARGO_PKG_VERSION"));
    println!("usage:");
    println!("  tech2ctl inspect <image.bin>   validate a Tech 2 card image");
    println!("  tech2ctl ports                 list candidate serial devices");
    println!("  tech2ctl mdb [cosids-dir]      inspect a TIS2000 data directory");
    println!("  tech2ctl tis export --cosids <dir> --out <dir>");
    println!("  tech2ctl tis probe --cosids <dir>");
    println!("  tech2ctl rpc <method> [json] [--socket PATH]");
    println!("  tech2ctl help");
}

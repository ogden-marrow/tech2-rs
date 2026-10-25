//! Local JSON-RPC daemon for the tech2-rs toolchain.
//!
//! The daemon owns the long-lived state: serial port discovery, image
//! inspection, and read-only TIS data queries. The GUI and CLI are clients.

use std::fs;
use std::io::{self, BufRead, BufReader, BufWriter, Write};
use std::os::unix::fs::{FileTypeExt, MetadataExt, PermissionsExt};
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::thread;

use serde_json::{json, Value};
use tech2_core::inspect_image;
use tech2_core::ports::list_ports;
use tech2_tis::mdb::MdbTools;
use tech2_tis::superbase;
use tech2_tis::{DataStore, SUPERBASE_DIRS};

/// Runtime state shared by request handlers.
#[derive(Debug, Clone)]
pub struct State {
    pub socket: PathBuf,
}

#[derive(Debug)]
struct RpcError {
    code: i64,
    message: String,
}

impl RpcError {
    fn new(code: i64, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
        }
    }

    fn method_not_found() -> Self {
        Self::new(-32601, "method not found")
    }

    fn invalid_params(message: impl Into<String>) -> Self {
        Self::new(-32602, message)
    }

    fn internal(message: impl Into<String>) -> Self {
        Self::new(-32000, message)
    }
}

/// Default Unix-socket path.
pub fn default_socket_path() -> PathBuf {
    if let Some(path) = std::env::var_os("TECH2D_SOCKET") {
        return PathBuf::from(path);
    }
    if let Some(dir) = std::env::var_os("XDG_RUNTIME_DIR") {
        return PathBuf::from(dir).join("tech2d.sock");
    }
    let uid = fs::metadata("/proc/self")
        .map(|meta| meta.uid())
        .unwrap_or(0);
    PathBuf::from(format!("/tmp/tech2d-{uid}.sock"))
}

/// Bind the socket and serve requests until the process is stopped.
pub fn serve(socket: &Path) -> io::Result<()> {
    if socket.exists() {
        let meta = fs::metadata(socket)?;
        if !meta.file_type().is_socket() {
            return Err(io::Error::new(
                io::ErrorKind::AlreadyExists,
                format!("{} exists and is not a socket", socket.display()),
            ));
        }
        fs::remove_file(socket)?;
    }
    let listener = UnixListener::bind(socket)?;
    fs::set_permissions(socket, fs::Permissions::from_mode(0o600))?;
    let state = Arc::new(State {
        socket: socket.to_path_buf(),
    });
    for connection in listener.incoming() {
        match connection {
            Ok(stream) => {
                let state = Arc::clone(&state);
                thread::spawn(move || {
                    if let Err(err) = handle_connection(stream, &state) {
                        eprintln!("tech2d connection: {err}");
                    }
                });
            }
            Err(err) => eprintln!("tech2d accept: {err}"),
        }
    }
    Ok(())
}

fn handle_connection(stream: UnixStream, state: &State) -> io::Result<()> {
    let reader = BufReader::new(stream.try_clone()?);
    let mut writer = BufWriter::new(stream);
    for line in reader.lines() {
        let line = line?;
        if line.trim().is_empty() {
            continue;
        }
        let response = handle_request(&line, state);
        writeln!(writer, "{response}")?;
        writer.flush()?;
    }
    Ok(())
}

fn handle_request(line: &str, state: &State) -> Value {
    let request: Value = match serde_json::from_str(line) {
        Ok(value) => value,
        Err(err) => return error_response(Value::Null, -32700, format!("parse error: {err}")),
    };
    let id = request.get("id").cloned().unwrap_or(Value::Null);
    let Some(method) = request.get("method").and_then(Value::as_str) else {
        return error_response(id, -32600, "invalid request: missing method".to_string());
    };
    let params = request.get("params").cloned().unwrap_or(Value::Null);
    match dispatch(method, &params, state) {
        Ok(result) => json!({"jsonrpc": "2.0", "id": id, "result": result}),
        Err(err) => error_response(id, err.code, err.message),
    }
}

fn error_response(id: Value, code: i64, message: String) -> Value {
    json!({
        "jsonrpc": "2.0",
        "id": id,
        "error": { "code": code, "message": message }
    })
}

fn dispatch(method: &str, params: &Value, state: &State) -> Result<Value, RpcError> {
    match method {
        "status" => Ok(json!({
            "name": "tech2d",
            "version": env!("CARGO_PKG_VERSION"),
            "socket": state.socket.display().to_string(),
            "capabilities": [
                "ports.list",
                "images.inspect",
                "tis.tables",
                "tis.probe",
                "card.capabilities"
            ],
            "native_card_writer": false
        })),
        "ports.list" => {
            let ports: Vec<Value> = list_ports()
                .iter()
                .map(|port| {
                    json!({
                        "device": port.device.display().to_string(),
                        "kind": port.kind
                    })
                })
                .collect();
            Ok(json!({ "ports": ports }))
        }
        "images.inspect" => {
            let path = require_str(params, "path")?;
            let car = params.get("car").and_then(Value::as_str).unwrap_or("9-5");
            let info =
                inspect_image(path, car).map_err(|err| RpcError::internal(err.to_string()))?;
            Ok(json!({
                "path": info.path.display().to_string(),
                "size": info.size,
                "size_mb": info.size_mb(),
                "version": info.version,
                "version_label": info.version_label,
                "magic_ok": info.magic_ok,
                "size_ok": info.size_ok,
                "valid": info.valid(),
                "coverage": info.coverage,
                "warnings": info.warnings,
                "display_name": info.display_name()
            }))
        }
        "tis.tables" => {
            let cosids = require_str(params, "cosids")?;
            let store =
                DataStore::open(cosids).map_err(|err| RpcError::internal(err.to_string()))?;
            let tools = MdbTools::from_env();
            if !tools.available() {
                return Err(RpcError::internal(
                    "mdbtools is not available; install it or set TECH2_MDBTOOLS",
                ));
            }
            let tables = tools
                .tables(&store.mdb_path())
                .map_err(|err| RpcError::internal(err.to_string()))?;
            Ok(json!({
                "cosids": store.root().display().to_string(),
                "mdb": store.mdb_path().display().to_string(),
                "count": tables.len(),
                "tables": tables
            }))
        }
        "tis.probe" => {
            let cosids = require_str(params, "cosids")?;
            let store =
                DataStore::open(cosids).map_err(|err| RpcError::internal(err.to_string()))?;
            let mut databases = Vec::new();
            for name in SUPERBASE_DIRS {
                let file = store.root().join(name).join("rfile000");
                if !file.is_file() {
                    databases.push(json!({ "database": name, "missing": true }));
                    continue;
                }
                let probe =
                    superbase::probe(&file).map_err(|err| RpcError::internal(err.to_string()))?;
                databases.push(json!({
                    "database": name,
                    "path": probe.path.display().to_string(),
                    "size": probe.size,
                    "magic_ok": probe.magic_ok,
                    "name": probe.name
                }));
            }
            Ok(json!({
                "cosids": store.root().display().to_string(),
                "databases": databases
            }))
        }
        "card.capabilities" => Ok(json!({
            "native_writer": false,
            "reason": "capture-driven protocol not implemented yet"
        })),
        "card.write" => Err(RpcError::new(
            -32000,
            "native card-writer protocol is not implemented yet; capture a write first",
        )),
        _ => Err(RpcError::method_not_found()),
    }
}

fn require_str<'a>(params: &'a Value, key: &str) -> Result<&'a str, RpcError> {
    params
        .get(key)
        .and_then(Value::as_str)
        .ok_or_else(|| RpcError::invalid_params(format!("missing string parameter: {key}")))
}

/// Send one JSON-RPC request and return its result.
pub fn rpc_call(socket: &Path, method: &str, params: Value) -> io::Result<Value> {
    let mut stream = UnixStream::connect(socket)?;
    let request = json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": method,
        "params": params
    });
    writeln!(stream, "{request}")?;
    stream.flush()?;
    let mut reader = BufReader::new(stream);
    let mut line = String::new();
    reader.read_line(&mut line)?;
    let response: Value = serde_json::from_str(&line)
        .map_err(|err| io::Error::new(io::ErrorKind::InvalidData, err))?;
    if let Some(error) = response.get("error") {
        return Err(io::Error::other(format!("RPC error: {error}")));
    }
    Ok(response.get("result").cloned().unwrap_or(Value::Null))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::File;
    use std::io::Write;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn temp_path(name: &str) -> PathBuf {
        let stamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        std::env::temp_dir().join(format!("tech2d-{stamp}-{name}"))
    }

    fn test_state() -> State {
        State {
            socket: PathBuf::from("/tmp/tech2d-test.sock"),
        }
    }

    #[test]
    fn status_request_works() {
        let response = handle_request(
            r#"{"jsonrpc":"2.0","id":1,"method":"status","params":{}}"#,
            &test_state(),
        );
        assert_eq!(response["result"]["name"], "tech2d");
        assert_eq!(response["result"]["native_card_writer"], false);
    }

    #[test]
    fn unknown_method_is_an_error() {
        let response = handle_request(
            r#"{"jsonrpc":"2.0","id":1,"method":"nope","params":{}}"#,
            &test_state(),
        );
        assert_eq!(response["error"]["code"], -32601);
    }

    #[test]
    fn inspects_a_synthetic_image() {
        let path = temp_path("Saab 148.bin");
        let mut file = File::create(&path).unwrap();
        file.write_all(b"T2  ").unwrap();
        file.set_len(tech2_core::THIRTY_TWO_MB).unwrap();
        let request = json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "images.inspect",
            "params": { "path": path.display().to_string() }
        });
        let response = handle_request(&request.to_string(), &test_state());
        assert_eq!(response["result"]["valid"], true);
        assert_eq!(response["result"]["version"], 148);
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn rpc_call_round_trips_over_a_socket() {
        let socket = temp_path("sock");
        let listener = UnixListener::bind(&socket).unwrap();
        let state = State {
            socket: socket.clone(),
        };
        let handle = thread::spawn(move || {
            if let Ok((stream, _)) = listener.accept() {
                handle_connection(stream, &state).unwrap();
            }
        });
        let result = rpc_call(&socket, "status", json!({})).unwrap();
        assert_eq!(result["name"], "tech2d");
        handle.join().unwrap();
        let _ = std::fs::remove_file(socket);
    }
}

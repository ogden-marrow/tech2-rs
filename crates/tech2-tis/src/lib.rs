//! Read-only probes for a TIS2000 installation.
//!
//! Nothing here writes to the source data. Importers and the SQLite index
//! arrive in later milestones.

use std::fs::File;
use std::io::{self, Read};
use std::path::{Path, PathBuf};
use std::process::Command;

pub mod mdb;
pub mod superbase;

/// SuperBase data directories commonly present in a TIS2000 cosids tree.
pub const SUPERBASE_DIRS: [&str; 4] = ["spssaabdb", "scasbdb", "sdt2sbdb", "vcsaabdb"];

/// A read-only view of a TIS2000 cosids directory.
#[derive(Debug, Clone)]
pub struct DataStore {
    root: PathBuf,
}

impl DataStore {
    pub fn open(root: impl Into<PathBuf>) -> io::Result<Self> {
        let root = std::fs::canonicalize(root.into()).map_err(|err| {
            io::Error::new(
                io::ErrorKind::NotFound,
                format!("not a TIS2000 cosids directory: {err}"),
            )
        })?;
        if !root.is_dir() {
            return Err(io::Error::new(
                io::ErrorKind::NotFound,
                format!("not a TIS2000 cosids directory: {}", root.display()),
            ));
        }
        Ok(Self { root })
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    pub fn mdb_path(&self) -> PathBuf {
        self.root.join("AppMan").join("COSIDS.MDB")
    }

    pub fn superbase_dirs(&self) -> Vec<PathBuf> {
        SUPERBASE_DIRS
            .iter()
            .map(|name| self.root.join(name))
            .collect()
    }

    /// True when the mdbtools CLI is available for read-only Access export.
    pub fn mdb_tools_available() -> bool {
        Command::new("mdb-tables")
            .arg("--version")
            .output()
            .map(|output| output.status.success())
            .unwrap_or(false)
    }
}

/// Minimal SuperBase header sniffing, useful for format analysis.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SuperBaseHeader {
    pub marker: Option<String>,
}

/// Read the first bytes of a SuperBase rfile and look for its record marker.
pub fn sniff_superbase(path: impl AsRef<Path>) -> io::Result<SuperBaseHeader> {
    let mut head = [0u8; 128];
    let mut file = File::open(path)?;
    let read = file.read(&mut head)?;
    let bytes = &head[..read];
    let marker = bytes
        .windows(b"SPS_SB_".len())
        .position(|window| window == b"SPS_SB_")
        .map(|start| {
            bytes[start..]
                .iter()
                .copied()
                .take_while(|byte| byte.is_ascii_graphic())
                .map(char::from)
                .collect::<String>()
        });
    Ok(SuperBaseHeader { marker })
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
        std::env::temp_dir().join(format!("tech2-tis-{stamp}-{name}"))
    }

    #[test]
    fn opens_a_directory_and_finds_expected_paths() {
        let path = temp_path("root");
        std::fs::create_dir_all(&path).unwrap();
        let store = DataStore::open(&path).unwrap();
        assert_eq!(store.superbase_dirs().len(), 4);
        assert!(store.mdb_path().ends_with("AppMan/COSIDS.MDB"));
        let _ = std::fs::remove_dir_all(path);
    }

    #[test]
    fn rejects_a_missing_directory() {
        assert!(DataStore::open("/definitely/not/a/real/tech2-dir").is_err());
    }

    #[test]
    fn sniffs_the_superbase_marker() {
        let path = temp_path("rfile000");
        let mut file = File::create(&path).unwrap();
        file.write_all(b"\0\0\0\0\0\0\0\0SPS_SB_080715_48114_H_On")
            .unwrap();
        let header = sniff_superbase(&path).unwrap();
        assert!(header.marker.unwrap().starts_with("SPS_SB_"));
        let _ = std::fs::remove_file(path);
    }
}

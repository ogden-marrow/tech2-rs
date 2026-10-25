//! SuperBase rfile header probes.
//!
//! This is format analysis, not a full SuperBase reader yet. It never writes.

use std::fs::File;
use std::io::{self, Read};
use std::path::{Path, PathBuf};

const MAGIC: [u8; 4] = [0xba, 0xdc, 0xcd, 0xab];

/// Header fields we can safely identify today.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SuperBaseProbe {
    pub path: PathBuf,
    pub size: u64,
    pub magic_ok: bool,
    pub name: Option<String>,
}

/// Read the first bytes of an rfile and extract the database name.
pub fn probe(path: impl AsRef<Path>) -> io::Result<SuperBaseProbe> {
    let path = path.as_ref().to_path_buf();
    let size = std::fs::metadata(&path)?.len();
    let mut head = [0u8; 256];
    let mut file = File::open(&path)?;
    let read = file.read(&mut head)?;
    let bytes = &head[..read];
    let magic_ok = bytes.len() >= 0x18 && bytes[0x14..0x18] == MAGIC;
    let name = find_name(bytes);
    Ok(SuperBaseProbe {
        path,
        size,
        magic_ok,
        name,
    })
}

fn find_name(bytes: &[u8]) -> Option<String> {
    let mut start = None;
    for (index, byte) in bytes.iter().enumerate().skip(0x20) {
        if byte.is_ascii_graphic() {
            if start.is_none() {
                start = Some(index);
            }
        } else if let Some(begin) = start {
            if index - begin >= 6 {
                return Some(String::from_utf8_lossy(&bytes[begin..index]).into_owned());
            }
            start = None;
        }
    }
    start.and_then(|begin| {
        if bytes.len() - begin >= 6 {
            Some(String::from_utf8_lossy(&bytes[begin..]).into_owned())
        } else {
            None
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::File;
    use std::io::Write;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn probes_a_synthetic_header() {
        let stamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let path = std::env::temp_dir().join(format!("tech2-sb-{stamp}.bin"));
        let mut header = vec![0u8; 64];
        header[0x14..0x18].copy_from_slice(&MAGIC);
        header[0x28..0x28 + 11].copy_from_slice(b"SPS_SB_0001");
        let mut file = File::create(&path).unwrap();
        file.write_all(&header).unwrap();
        let probe = probe(&path).unwrap();
        assert!(probe.magic_ok);
        assert_eq!(probe.name.as_deref(), Some("SPS_SB_0001"));
        let _ = std::fs::remove_file(path);
    }
}

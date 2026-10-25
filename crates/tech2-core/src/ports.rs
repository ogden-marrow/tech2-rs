//! Linux serial-port discovery shared by the CLI and daemon.

use std::fs;
use std::path::PathBuf;

/// A candidate serial device.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PortInfo {
    pub device: PathBuf,
    pub kind: &'static str,
}

/// Classify a /dev entry name.
pub fn kind_for(name: &str) -> Option<&'static str> {
    if name.starts_with("ttyUSB") {
        Some("USB serial")
    } else if name.starts_with("ttyACM") {
        Some("ACM serial")
    } else if matches!(name, "ttyS0" | "ttyS1" | "ttyS2" | "ttyS3") {
        Some("built-in serial")
    } else {
        None
    }
}

/// List candidate serial ports under /dev, sorted by device path.
pub fn list_ports() -> Vec<PortInfo> {
    let Ok(entries) = fs::read_dir("/dev") else {
        return Vec::new();
    };
    let mut ports = Vec::new();
    for entry in entries.flatten() {
        let name = entry.file_name().to_string_lossy().to_string();
        if let Some(kind) = kind_for(&name) {
            ports.push(PortInfo {
                device: entry.path(),
                kind,
            });
        }
    }
    ports.sort_by(|a, b| a.device.cmp(&b.device));
    ports
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classifies_expected_devices() {
        assert_eq!(kind_for("ttyUSB0"), Some("USB serial"));
        assert_eq!(kind_for("ttyACM1"), Some("ACM serial"));
        assert_eq!(kind_for("ttyS3"), Some("built-in serial"));
        assert_eq!(kind_for("ttyS4"), None);
        assert_eq!(kind_for("sda"), None);
    }
}

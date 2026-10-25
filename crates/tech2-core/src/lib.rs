//! Image model and validation for Saab/GM Tech 2 card images.

use std::fs::File;
use std::io::{self, Read};
use std::path::{Path, PathBuf};

/// A 10 MB linear-flash card image.
pub const TEN_MB: u64 = 10 * 1024 * 1024;
/// A 32 MB linear-flash card image.
pub const THIRTY_TWO_MB: u64 = 32 * 1024 * 1024;

/// Valid raw card-image sizes.
pub const VALID_SIZES: [u64; 2] = [TEN_MB, THIRTY_TWO_MB];

/// Coverage hint for a known Saab card version.
pub fn coverage_for(version: u16) -> Option<&'static str> {
    match version {
        2 => Some("Saab 9-3 diagnostics; DTC C0131 08 and C0252 (ESP)"),
        44 => Some("Saab 900 1990-1998 and 9000 1988-1998"),
        135 => Some("9-3 1998-2003, 9-3 Sport 2003-2008, 9-5 1998-2008"),
        136 => Some("9-3 1998-2003, 9-3 Sport 2003-2009, 9-5 1998-2009"),
        143 => Some("9-3 1998-2003, 9-3 Sport 2003-2010, 9-5 1998-2010"),
        144 => Some("9-3 1998-2003, 9-3 Sport 2003-2010, 9-5 1998-2010"),
        145 => Some("9-3 1998-2003, 9-3 Sport 2003-2011, 9-5 1998-2010"),
        146 => Some("9-3 1998-2003, 9-3 Sport 2003-2011, 9-5 1998-2010"),
        147 => Some("9-3 1998-2003, 9-3 Sport 2003-2011, 9-5 1998-2010"),
        148 => Some("9-3 1998-2003, 9-3 Sport 2003-2011, 9-5 1998-2010"),
        _ => None,
    }
}

/// Everything we can learn from a card image without writing it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ImageInfo {
    pub path: PathBuf,
    pub size: u64,
    pub version: Option<u16>,
    pub version_label: String,
    pub magic_ok: bool,
    pub size_ok: bool,
    pub coverage: Option<&'static str>,
    pub warnings: Vec<String>,
}

impl ImageInfo {
    pub fn valid(&self) -> bool {
        self.magic_ok && self.size_ok
    }

    pub fn size_mb(&self) -> f64 {
        self.size as f64 / (1024.0 * 1024.0)
    }

    pub fn display_name(&self) -> String {
        match self.version {
            Some(_) => format!("Saab {} · {:.0} MB", self.version_label, self.size_mb()),
            None => format!(
                "{} · {:.0} MB",
                self.path
                    .file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("card image"),
                self.size_mb()
            ),
        }
    }
}

/// Inspect a raw .bin image. car is used only for compatibility warnings.
pub fn inspect_image(path: impl AsRef<Path>, car: &str) -> io::Result<ImageInfo> {
    let path = path.as_ref().to_path_buf();
    let size = std::fs::metadata(&path)?.len();
    let mut head = [0u8; 16];
    let mut file = File::open(&path)?;
    let read = file.read(&mut head)?;
    let magic_ok = read >= 4 && &head[..4] == b"T2  ";

    let parsed = parse_version(
        path.file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or_default(),
    );
    let (version, version_label) = match parsed {
        Some((major, Some(minor))) => (Some(major), format!("{major}.{minor:03}")),
        Some((major, None)) => (Some(major), format!("{major}.000")),
        None => (None, "unknown".to_string()),
    };

    let size_ok = VALID_SIZES.contains(&size);
    let expected_mb = if version == Some(44) { 10 } else { 32 };
    let mut warnings = Vec::new();

    if !magic_ok {
        warnings.push("Missing the T2 header; this may not be a card image.".to_string());
    }
    if !size_ok {
        warnings.push(format!(
            "Unexpected size {:.0} MB; Tech 2 images are 10 MB or 32 MB.",
            size as f64 / (1024.0 * 1024.0)
        ));
    } else if (size / (1024 * 1024)) as i32 != expected_mb {
        warnings.push(format!(
            "This looks like a {expected_mb} MB image, but the file is {:.0} MB.",
            size as f64 / (1024.0 * 1024.0)
        ));
    }
    if version == Some(44) && car == "9-5" {
        warnings.push("v44.000 is for the 900/9000, not the 9-5.".to_string());
    }
    if version == Some(143) {
        warnings.push("v143.000 is English-only.".to_string());
    }
    if matches!(version, Some(v) if v >= 144) {
        warnings.push("This image requires a 32 MB PCMCIA card.".to_string());
    }
    match version {
        None => warnings.push(
            "Could not read a version from the filename; verify this is the right image."
                .to_string(),
        ),
        Some(v) if coverage_for(v).is_none() => {
            warnings.push("Unknown Saab version; verify this image matches your car.".to_string());
        }
        Some(_) => {}
    }

    Ok(ImageInfo {
        path,
        size,
        version,
        version_label,
        magic_ok,
        size_ok,
        coverage: version.and_then(coverage_for),
        warnings,
    })
}

/// Find the last digit group in a filename, e.g. Saab 146.000.bin -> (146, Some(0)).
fn parse_version(stem: &str) -> Option<(u16, Option<u16>)> {
    let mut last = None;
    for part in stem.split(|c: char| !c.is_ascii_digit() && c != '.') {
        if part.is_empty() {
            continue;
        }
        let mut pieces = part.split('.');
        let major = match pieces.next().and_then(|s| s.parse::<u16>().ok()) {
            Some(v) if (2..=999).contains(&v) => v,
            _ => continue,
        };
        let minor = pieces.next().and_then(|s| s.parse::<u16>().ok());
        last = Some((major, minor));
    }
    last
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::File;
    use std::io::Write;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn temp_image(size: u64, name: &str, magic: &[u8]) -> PathBuf {
        let stamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let path = std::env::temp_dir().join(format!("tech2-core-{stamp}-{name}"));
        let mut file = File::create(&path).unwrap();
        file.write_all(magic).unwrap();
        file.set_len(size).unwrap();
        path
    }

    #[test]
    fn accepts_saab_148() {
        let path = temp_image(THIRTY_TWO_MB, "Saab 148.bin", b"T2  ");
        let info = inspect_image(&path, "9-5").unwrap();
        assert!(info.valid());
        assert_eq!(info.version, Some(148));
        assert!(info.display_name().contains("148"));
        assert!(info.coverage.unwrap().contains("9-5"));
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn warns_about_v44_on_a_9_5() {
        let path = temp_image(TEN_MB, "Saab 44.bin", b"T2  ");
        let info = inspect_image(&path, "9-5").unwrap();
        assert!(info.valid());
        assert_eq!(info.version, Some(44));
        assert!(info.warnings.iter().any(|w| w.contains("not the 9-5")));
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn rejects_bad_magic() {
        let path = temp_image(THIRTY_TWO_MB, "Saab 148.bin", b"NOPE");
        let info = inspect_image(&path, "9-5").unwrap();
        assert!(!info.magic_ok);
        assert!(!info.valid());
        assert!(info.warnings.iter().any(|w| w.contains("T2")));
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn rejects_wrong_size() {
        let path = temp_image(1024, "Saab 148.bin", b"T2  ");
        let info = inspect_image(&path, "9-5").unwrap();
        assert!(!info.size_ok);
        assert!(!info.valid());
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn parses_versions() {
        assert_eq!(parse_version("Saab 146.000"), Some((146, Some(0))));
        assert_eq!(parse_version("SAAB 148"), Some((148, None)));
        assert_eq!(parse_version("gmnao33.004"), Some((33, Some(4))));
        assert_eq!(parse_version("random"), None);
    }
}

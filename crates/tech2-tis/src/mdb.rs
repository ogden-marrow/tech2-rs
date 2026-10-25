//! Read-only Access/Jet export through the mdbtools CLI.

use std::env;
use std::io;
use std::path::{Path, PathBuf};
use std::process::Command;

/// Locate and run the read-only mdbtools utilities.
#[derive(Debug, Clone)]
pub struct MdbTools {
    bin_dir: Option<PathBuf>,
}

impl MdbTools {
    /// Use TECH2_MDBTOOLS when set, otherwise look on PATH.
    pub fn from_env() -> Self {
        Self {
            bin_dir: env::var_os("TECH2_MDBTOOLS").map(PathBuf::from),
        }
    }

    pub fn with_bin_dir(dir: impl Into<PathBuf>) -> Self {
        Self {
            bin_dir: Some(dir.into()),
        }
    }

    fn tool(&self, name: &str) -> PathBuf {
        match &self.bin_dir {
            Some(dir) => dir.join(name),
            None => PathBuf::from(name),
        }
    }

    /// True when the mdbtools CLI appears to be runnable.
    pub fn available(&self) -> bool {
        Command::new(self.tool("mdb-tables"))
            .arg("--help")
            .output()
            .is_ok()
    }

    /// List tables in an Access database.
    pub fn tables(&self, db: &Path) -> io::Result<Vec<String>> {
        let output = Command::new(self.tool("mdb-tables"))
            .arg("-1")
            .arg(db)
            .output()?;
        if !output.status.success() {
            return Err(io::Error::other(format!(
                "mdb-tables failed: {}",
                String::from_utf8_lossy(&output.stderr).trim()
            )));
        }
        Ok(String::from_utf8_lossy(&output.stdout)
            .lines()
            .map(str::trim)
            .filter(|line| !line.is_empty())
            .map(str::to_string)
            .collect())
    }

    /// Print the database schema.
    pub fn schema(&self, db: &Path) -> io::Result<String> {
        self.run_text("mdb-schema", &[db.as_os_str()])
    }

    /// Export one table as CSV. The source database is opened read-only by mdbtools.
    pub fn export_table(&self, db: &Path, table: &str) -> io::Result<String> {
        self.run_text("mdb-export", &[db.as_os_str(), table.as_ref()])
    }

    fn run_text(&self, tool: &str, args: &[&std::ffi::OsStr]) -> io::Result<String> {
        let output = Command::new(self.tool(tool)).args(args).output()?;
        if !output.status.success() {
            return Err(io::Error::other(format!(
                "{tool} failed: {}",
                String::from_utf8_lossy(&output.stderr).trim()
            )));
        }
        Ok(String::from_utf8_lossy(&output.stdout).into_owned())
    }
}

/// Turn a table name into a safe file stem.
pub fn sanitize_table_name(name: &str) -> String {
    let mut out = String::with_capacity(name.len());
    for ch in name.chars() {
        if ch.is_ascii_alphanumeric() || ch == '_' || ch == '-' {
            out.push(ch);
        } else {
            out.push('_');
        }
    }
    if out.is_empty() {
        "table".to_string()
    } else {
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sanitizes_table_names() {
        assert_eq!(
            sanitize_table_name("APPMAccessPermission"),
            "APPMAccessPermission"
        );
        assert_eq!(sanitize_table_name("a/b c"), "a_b_c");
        assert_eq!(sanitize_table_name(""), "table");
    }
}

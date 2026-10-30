//! Wo die Anwendung ihre Dateien ablegt.
//!
//! Die Fallunterscheidung zwischen macOS und Linux steht **nur hier**. Wer
//! anderswo `#[cfg(target_os = ...)]` für einen Pfad schreibt, hat sich
//! verlaufen.
//!
//! | | macOS | Linux |
//! |---|---|---|
//! | Daten | `~/Library/Application Support/starter` | `~/.local/share/starter` |
//! | Konfiguration | `~/Library/Application Support/starter` | `~/.config/starter` |
//!
//! Beide Verzeichnisse lassen sich über Umgebungsvariablen verlegen. Das ist
//! kein Debug-Hintertürchen, sondern die Grundlage dafür, dass die
//! Abnahmetests der CLI auf einem Wegwerfverzeichnis laufen können.

use std::path::{Path, PathBuf};

use crate::error::{Error, Result};

/// Anwendungsname. Bestimmt Verzeichnisnamen und wird von
/// `scripts/new-project.sh` ersetzt.
pub const APP_NAME: &str = "starter";

/// Umgebungsvariablen, die die Verzeichnisse überschreiben.
pub const ENV_DATA_DIR: &str = "STARTER_DATA_DIR";
pub const ENV_CONFIG_DIR: &str = "STARTER_CONFIG_DIR";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Paths {
    data_dir: PathBuf,
    config_dir: PathBuf,
}

impl Paths {
    /// Die echten Pfade der Plattform, gegebenenfalls durch die
    /// Umgebungsvariablen überschrieben.
    pub fn resolve() -> Result<Self> {
        let dirs = directories::ProjectDirs::from("", "", APP_NAME).ok_or(Error::NoHomeDir)?;

        let data_dir = match std::env::var_os(ENV_DATA_DIR) {
            Some(v) => PathBuf::from(v),
            None => dirs.data_dir().to_path_buf(),
        };
        let config_dir = match std::env::var_os(ENV_CONFIG_DIR) {
            Some(v) => PathBuf::from(v),
            None => dirs.config_dir().to_path_buf(),
        };

        Ok(Self {
            data_dir,
            config_dir,
        })
    }

    /// Zwei ausdrücklich gewählte Verzeichnisse.
    ///
    /// Für Aufrufer, die die Wahl bereits getroffen haben — etwa die
    /// Kommandozeile mit `--data-dir`.
    pub fn from_parts(data_dir: PathBuf, config_dir: PathBuf) -> Self {
        Self {
            data_dir,
            config_dir,
        }
    }

    /// Alles unterhalb eines Wurzelverzeichnisses. Für Tests.
    ///
    /// Bewusst kein Umweg über Umgebungsvariablen: `cargo test` lässt Tests
    /// nebenläufig laufen, und `std::env::set_var` gilt für den ganzen Prozess.
    pub fn under(root: impl AsRef<Path>) -> Self {
        let root = root.as_ref();
        Self {
            data_dir: root.join("data"),
            config_dir: root.join("config"),
        }
    }

    pub fn data_dir(&self) -> &Path {
        &self.data_dir
    }

    pub fn config_dir(&self) -> &Path {
        &self.config_dir
    }

    /// Die SQLite-Datei. WAL legt daneben `-wal` und `-shm` an.
    pub fn db_file(&self) -> PathBuf {
        self.data_dir.join(format!("{APP_NAME}.sqlite3"))
    }

    pub fn config_file(&self) -> PathBuf {
        self.config_dir.join("config.toml")
    }

    pub fn log_file(&self) -> PathBuf {
        self.data_dir.join("log").join(format!("{APP_NAME}.log"))
    }

    /// Legt alle Verzeichnisse an. Mehrfaches Aufrufen ist harmlos.
    pub fn ensure(&self) -> Result<()> {
        for dir in [
            self.data_dir.as_path(),
            self.config_dir.as_path(),
            &self.data_dir.join("log"),
        ] {
            std::fs::create_dir_all(dir).map_err(|source| Error::Io {
                path: dir.to_path_buf(),
                source,
            })?;
        }
        Ok(())
    }
}

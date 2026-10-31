//! Fachlogik des Kits.
//!
//! # Die eine wichtige Regel
//!
//! Diese Kiste kennt **weder Kommandozeile noch Oberfläche noch stdout**.
//! Sie nimmt Werte entgegen, gibt Werte oder [`Error`] zurück und schreibt
//! nichts auf den Bildschirm. Wer hier `clap`, `tauri`, `println!` oder
//! `std::process::exit` einbaut, bringt `crates/core/tests/layering.rs` zum
//! Fehlschlagen — und das ist der Zweck dieses Tests.
//!
//! Der Grund ist nicht Sauberkeit um ihrer selbst willen: Alles, was hier
//! steht, gilt für die Kommandozeile **und** die Oberfläche gleichermaßen.
//! Eine Regel, die in der einen Schale steht, fehlt der anderen.
//!
//! # Einstieg
//!
//! ```no_run
//! use teacher_class_core::{App, Paths, class::{self, ClassInput}, student};
//!
//! let app = App::open(Paths::resolve()?)?;
//! let created = class::create(app.conn(), ClassInput::new("7a", 2025))?;
//! student::add_many(app.conn(), created.id, "Muster, Max\nBeispiel, Eva")?;
//! let all = class::list(app.conn())?;
//! # Ok::<(), teacher_class_core::Error>(())
//! ```

pub mod class;
pub mod config;
pub mod db;
pub mod error;
pub mod exam;
pub mod export;
pub mod grading;
pub mod ids;
pub mod paths;
pub mod student;
pub mod timestamp;

pub use config::{Config, Theme};
pub use error::{Error, ErrorCode, FieldError, Result, ValidationError};
pub use paths::Paths;

use std::path::Path;

use rusqlite::Connection;

/// Alles, was eine laufende Anwendung braucht: Pfade, Einstellungen, Datenbank.
///
/// Beide Schalen bauen sich beim Start genau eines davon.
pub struct App {
    paths: Paths,
    config: Config,
    conn: Connection,
}

impl App {
    /// Legt Verzeichnisse an, liest die Einstellungen und öffnet die Datenbank.
    ///
    /// `paths` sind die Pfade der Plattform; ein gewählter Ort aus den
    /// Einstellungen kommt dazu. Am Standardort wird die Datenbank bei Bedarf
    /// angelegt (Erststart). Am **gewählten** Ort nicht: Fehlt dort die Datei,
    /// etwa weil der USB-Stick fehlt, gibt es [`Error::DatabaseMissing`] —
    /// still eine leere Datenbank anzulegen sähe nach verlorenen Daten aus.
    pub fn open(paths: Paths) -> Result<Self> {
        paths.ensure()?;
        let config = Config::load(&paths.config_file())?;
        let paths = paths.for_config(&config);
        let file = paths.db_file();
        if config.db_file.is_some() && !file.is_file() {
            return Err(Error::DatabaseMissing { path: file });
        }
        let conn = db::open(&file)?;
        Ok(Self {
            paths,
            config,
            conn,
        })
    }

    /// Nimmt eine vorhandene Datenbankdatei als Speicherort und merkt ihn in
    /// den Einstellungen. `base` sind die Pfade der Plattform.
    pub fn use_database(base: &Paths, file: &Path) -> Result<Self> {
        if !file.is_file() {
            return Err(db_file_error("Hier liegt keine Datei"));
        }
        Self::switch(base, Some(file))
    }

    /// Legt eine neue, leere Datenbank an und nimmt sie als Speicherort. Eine
    /// vorhandene Datei wird nie überschrieben.
    pub fn create_database(base: &Paths, file: &Path) -> Result<Self> {
        if file.exists() {
            return Err(db_file_error(
                "Hier liegt schon eine Datei — dafür „Vorhandene Datenbank wählen“ nehmen",
            ));
        }
        Self::switch(base, Some(file))
    }

    /// Zurück zum Standardort. Die Datenbank dort wird bei Bedarf angelegt.
    pub fn use_default_database(base: &Paths) -> Result<Self> {
        Self::switch(base, None)
    }

    fn switch(base: &Paths, file: Option<&Path>) -> Result<Self> {
        base.ensure()?;
        let mut config = Config::load(&base.config_file())?;
        config.db_file = file.map(|f| f.to_string_lossy().into_owned());
        config.validate()?;
        let paths = base.clone().for_config(&config);
        // Erst öffnen, dann merken: Scheitert das Öffnen (keine SQLite-Datei,
        // kein Schreibrecht), bleibt der bisherige Ort eingestellt.
        let conn = db::open(&paths.db_file())?;
        config.save(&base.config_file())?;
        Ok(Self {
            paths,
            config,
            conn,
        })
    }

    /// Eine Anwendung ohne Datei auf der Platte, für Tests.
    ///
    /// Die Pfade zeigen ins Leere — wer in einem Test Dateien braucht, nimmt
    /// [`App::open`] mit [`Paths::under`] auf einem `tempfile::TempDir`.
    pub fn in_memory() -> Result<Self> {
        Ok(Self {
            paths: Paths::under(std::env::temp_dir().join("teacher-class-in-memory")),
            config: Config::default(),
            conn: db::open_in_memory()?,
        })
    }

    pub fn conn(&self) -> &Connection {
        &self.conn
    }

    pub fn paths(&self) -> &Paths {
        &self.paths
    }

    pub fn config(&self) -> &Config {
        &self.config
    }

    /// Prüft, speichert und übernimmt neue Einstellungen.
    ///
    /// Bei ungültigen Werten bleibt der alte Stand unverändert — eine
    /// halbübernommene Konfiguration wäre schlimmer als eine abgelehnte.
    pub fn set_config(&mut self, config: Config) -> Result<()> {
        config.validate()?;
        if config.db_file != self.config.db_file {
            return Err(db_file_error(
                "lässt sich hier nicht ändern — „Datenbank wählen“ nehmen",
            ));
        }
        self.paths.ensure()?;
        config.save(&self.paths.config_file())?;
        self.config = config;
        Ok(())
    }
}

/// Ein Eingabefehler am Feld `db_file`.
fn db_file_error(message: &str) -> Error {
    let mut v = error::Validator::new();
    v.require(false, "db_file", message);
    v.finish().expect_err("require(false) schlägt immer fehl")
}

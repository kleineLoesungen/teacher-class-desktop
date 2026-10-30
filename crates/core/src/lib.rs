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
//! use starter_core::{App, Paths, note::{self, NoteInput, Order}};
//!
//! let app = App::open(Paths::resolve()?)?;
//! let created = note::create(app.conn(), NoteInput::new("Titel", "Text"))?;
//! let all = note::list(app.conn(), Order::Recent)?;
//! # Ok::<(), starter_core::Error>(())
//! ```

pub mod config;
pub mod db;
pub mod error;
pub mod note;
pub mod paths;
pub mod timestamp;

pub use config::{Config, Theme};
pub use error::{Error, ErrorCode, FieldError, Result, ValidationError};
pub use paths::Paths;

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
    pub fn open(paths: Paths) -> Result<Self> {
        paths.ensure()?;
        let config = Config::load(&paths.config_file())?;
        let conn = db::open(&paths.db_file())?;
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
            paths: Paths::under(std::env::temp_dir().join("starter-in-memory")),
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
        self.paths.ensure()?;
        config.save(&self.paths.config_file())?;
        self.config = config;
        Ok(())
    }
}

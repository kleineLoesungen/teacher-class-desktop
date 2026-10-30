//! Fehler der Fachlogik.
//!
//! Zwei Sorten, und der Unterschied ist wichtig:
//!
//! * [`ValidationError`] ist die Schuld des Benutzers. Er enthält je Feld eine
//!   Meldung, die eine Oberfläche direkt neben das Eingabefeld schreiben kann.
//! * Alles andere ist die Schuld des Programms oder der Umgebung.
//!
//! In den Schalen (`cli`, `desktop`) wird daraus `anyhow::Error`. Fachlogik gibt
//! niemals `anyhow` zurück — sonst kann der Aufrufer die beiden Sorten nicht
//! mehr auseinanderhalten.

use std::path::PathBuf;

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("nicht gefunden")]
    NotFound,

    // Kein #[from]: das würde den inneren Fehler zur Ursache machen, und
    // anyhow druckte dieselbe Meldung zweimal ("Fehler: X / Ursache: X").
    #[error("{0}")]
    Validation(ValidationError),

    #[error("Datenbankfehler: {0}")]
    Db(#[from] rusqlite::Error),

    #[error("Migration fehlgeschlagen: {0}")]
    Migration(#[from] rusqlite_migration::Error),

    #[error("{path}: {source}")]
    Io {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },

    #[error("{path} ist keine gültige Konfiguration: {source}")]
    ConfigParse {
        path: PathBuf,
        #[source]
        source: toml::de::Error,
    },

    #[error("kein Heimatverzeichnis gefunden — ist HOME gesetzt?")]
    NoHomeDir,

    #[error("Datensatz beschädigt: {0}")]
    Corrupt(String),

    #[error("{prefix:?} passt auf {} Datensätze — mehr Zeichen angeben", .matches.len())]
    Ambiguous {
        prefix: String,
        matches: Vec<String>,
    },
}

/// Maschinenlesbare Fehlerart.
///
/// Dieselben Namen stehen im JSON der Kommandozeile (`error.code`) und in der
/// Fehlerhülle der Desktop-Schale. Ein Skript und die Oberfläche können sich
/// darauf verlassen — ein Name wird deshalb nie umbenannt, nur ergänzt.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, ts_rs::TS)]
#[serde(rename_all = "snake_case")]
pub enum ErrorCode {
    NotFound,
    ValidationFailed,
    AmbiguousId,
    ConfigInvalid,
    InternalError,
}

impl ErrorCode {
    pub fn as_str(self) -> &'static str {
        match self {
            ErrorCode::NotFound => "not_found",
            ErrorCode::ValidationFailed => "validation_failed",
            ErrorCode::AmbiguousId => "ambiguous_id",
            ErrorCode::ConfigInvalid => "config_invalid",
            ErrorCode::InternalError => "internal_error",
        }
    }
}

impl Error {
    pub fn code(&self) -> ErrorCode {
        match self {
            Error::NotFound => ErrorCode::NotFound,
            Error::Validation(_) => ErrorCode::ValidationFailed,
            Error::Ambiguous { .. } => ErrorCode::AmbiguousId,
            Error::ConfigParse { .. } => ErrorCode::ConfigInvalid,
            _ => ErrorCode::InternalError,
        }
    }

    /// Die Feldfehler, falls es ein Eingabefehler war. Sonst `None`.
    ///
    /// Damit muss keine Schale `match` auf die Variante schreiben.
    pub fn fields(&self) -> Option<&[FieldError]> {
        match self {
            Error::Validation(v) => Some(&v.fields),
            _ => None,
        }
    }
}

/// Eine Meldung, die zu genau einem Eingabefeld gehört.
///
/// `field` ist englisch und entspricht dem Feldnamen im Modell (`title`,
/// `body`). `message` ist deutsch und für Menschen.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, ts_rs::TS)]
pub struct FieldError {
    pub field: &'static str,
    pub message: String,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct ValidationError {
    pub fields: Vec<FieldError>,
}

impl std::fmt::Display for ValidationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut first = true;
        for e in &self.fields {
            if !first {
                write!(f, "; ")?;
            }
            write!(f, "{}: {}", e.field, e.message)?;
            first = false;
        }
        Ok(())
    }
}

impl std::error::Error for ValidationError {}

/// Sammelt Feldfehler und wird am Ende der Prüfung ausgewertet.
///
/// Wichtig: alle Felder prüfen, dann einmal abbrechen. Wer beim ersten Fehler
/// zurückkehrt, zwingt den Benutzer, dieselbe Maske mehrfach abzuschicken.
#[derive(Debug, Default)]
pub struct Validator {
    fields: Vec<FieldError>,
}

impl Validator {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add(&mut self, field: &'static str, message: impl Into<String>) {
        self.fields.push(FieldError {
            field,
            message: message.into(),
        });
    }

    /// Fügt den Fehler nur hinzu, wenn `ok` falsch ist.
    pub fn require(&mut self, ok: bool, field: &'static str, message: impl Into<String>) {
        if !ok {
            self.add(field, message);
        }
    }

    pub fn finish(self) -> Result<()> {
        if self.fields.is_empty() {
            Ok(())
        } else {
            Err(Error::Validation(ValidationError {
                fields: self.fields,
            }))
        }
    }
}

impl From<ValidationError> for Error {
    fn from(v: ValidationError) -> Self {
        Error::Validation(v)
    }
}

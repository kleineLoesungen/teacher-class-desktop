//! Die Fehlerhülle über die IPC-Grenze.
//!
//! Ein Tauri-Befehl gibt `Result<T, E>` zurück; im Webview wird aus `Err(E)`
//! ein abgelehntes Promise mit `E` als JSON. `E` muss deshalb `Serialize`
//! sein — `starter_core::Error` ist es absichtlich nicht, denn darin stecken
//! `rusqlite::Error` und Pfade.
//!
//! Die Form ist dieselbe wie `error` im JSON der Kommandozeile:
//!
//! ```json
//! { "code": "validation_failed", "message": "title: darf nicht leer sein",
//!   "fields": [{ "field": "title", "message": "darf nicht leer sein" }] }
//! ```
//!
//! Die Oberfläche unterscheidet wie die Kommandozeile zwei Sorten: Hat der
//! Fehler `fields`, gehört die Meldung ans Feld. Sonst oben ins Fenster.

use serde::Serialize;
use starter_core::{ErrorCode, FieldError};
use ts_rs::TS;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
pub struct ApiError {
    pub code: ErrorCode,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub fields: Option<Vec<FieldError>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub matches: Option<Vec<String>>,
}

impl ApiError {
    /// Ein Fehler des Programms, nicht der Eingabe.
    pub fn internal(message: impl Into<String>) -> Self {
        Self {
            code: ErrorCode::InternalError,
            message: message.into(),
            fields: None,
            matches: None,
        }
    }
}

impl From<starter_core::Error> for ApiError {
    fn from(err: starter_core::Error) -> Self {
        // Programmfehler landen zusätzlich im Protokoll. Eingabefehler nicht —
        // die sind Alltag und würden die Datei nur füllen.
        if err.code() == ErrorCode::InternalError {
            tracing::error!(error = %err, "Befehl fehlgeschlagen");
        }
        let matches = match &err {
            starter_core::Error::Ambiguous { matches, .. } => Some(matches.clone()),
            _ => None,
        };
        Self {
            code: err.code(),
            message: err.to_string(),
            fields: err.fields().map(<[FieldError]>::to_vec),
            matches,
        }
    }
}

pub type ApiResult<T> = Result<T, ApiError>;

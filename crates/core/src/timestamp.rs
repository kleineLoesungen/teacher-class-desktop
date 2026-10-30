//! Zeitstempel.
//!
//! **Gespeichert wird ausschließlich UTC**, als RFC-3339-Text. Umgerechnet
//! wird erst bei der Anzeige, in der Schale. Wer in der Fachlogik lokale Zeit
//! speichert, erzeugt Datensätze, die nach einem Umzug in eine andere Zeitzone
//! falsch sind.
//!
//! Text statt Zahl, weil `sqlite3 starter.sqlite3 "select * from notes"` dann
//! lesbar ist. Sortieren lässt sich RFC 3339 in UTC trotzdem korrekt, weil das
//! Format lexikografisch aufsteigt.

use time::OffsetDateTime;
use time::format_description::well_known::Rfc3339;

use crate::error::{Error, Result};

pub fn now() -> OffsetDateTime {
    OffsetDateTime::now_utc()
}

pub fn to_text(t: OffsetDateTime) -> String {
    t.to_offset(time::UtcOffset::UTC)
        .format(&Rfc3339)
        .expect("RFC 3339 kann jeden Zeitstempel darstellen")
}

pub fn from_text(s: &str) -> Result<OffsetDateTime> {
    OffsetDateTime::parse(s, &Rfc3339)
        .map_err(|e| Error::Corrupt(format!("Zeitstempel {s:?} ist unlesbar: {e}")))
}

/// Für die Anzeige: UTC in die Zeitzone des Betrachters umrechnen.
///
/// Scheitert die Abfrage der Zeitzone — das passiert in manchen Containern und
/// in nebenläufigen Tests —, bleibt es bei UTC. Lieber eine Stunde daneben als
/// ein Absturz.
pub fn to_local(t: OffsetDateTime) -> OffsetDateTime {
    match time::UtcOffset::current_local_offset() {
        Ok(offset) => t.to_offset(offset),
        Err(_) => t,
    }
}

/// Zeitstempel in JSON als RFC-3339-Text, nicht als Zahl.
///
/// Verwendung: `#[serde(with = "crate::timestamp::serde_rfc3339")]`.
pub mod serde_rfc3339 {
    use serde::Serializer;
    use time::OffsetDateTime;

    pub fn serialize<S: Serializer>(t: &OffsetDateTime, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_str(&super::to_text(*t))
    }
}

//! Kennungen: Kurzform und Auflösung.
//!
//! Die Kennungen sind UUIDv7. Die **beginnt mit dem Zeitstempel** — zwei
//! Datensätze aus derselben Sekunde teilen sich die ersten zwölf Zeichen.
//! Darum gilt die Kurzform dem **Ende**, nicht dem Anfang.

use rusqlite::Connection;
use uuid::Uuid;

use crate::error::{Error, Result};

/// Länge der Kurzform. Acht Hexziffern sind 32 Bit Zufall.
pub const SHORT_ID_LEN: usize = 8;

/// Die Tabellen, deren Kennungen sich auflösen lassen. Ein fester Satz statt
/// eines Textes, damit nie ein Tabellenname von außen im SQL landet.
#[derive(Debug, Clone, Copy)]
pub enum Table {
    Classes,
    Students,
}

impl Table {
    fn name(self) -> &'static str {
        match self {
            Table::Classes => "classes",
            Table::Students => "students",
        }
    }
}

/// Die letzten acht Zeichen einer Kennung.
pub fn short_id(id: Uuid) -> String {
    let full = id.to_string();
    full[full.len() - SHORT_ID_LEN..].to_string()
}

/// Findet die Kennung zu einer vollständigen UUID oder einem eindeutigen Ende.
///
/// Ist das Endstück nicht eindeutig, gibt es [`Error::Ambiguous`] statt eines
/// zufälligen Treffers.
pub fn resolve(conn: &Connection, table: Table, needle: &str) -> Result<Uuid> {
    let needle = needle.trim();
    if needle.is_empty() {
        return Err(Error::NotFound);
    }
    // Nur Hexziffern und Bindestriche: aus einem "%" wird so kein Joker.
    if !needle.chars().all(|c| c.is_ascii_hexdigit() || c == '-') {
        return Err(Error::NotFound);
    }

    let sql = format!(
        "SELECT id FROM {} WHERE id LIKE ?1 ORDER BY id LIMIT 10",
        table.name()
    );
    let mut stmt = conn.prepare(&sql)?;
    let matches: Vec<String> = stmt
        .query_map([format!("%{needle}")], |r| r.get::<_, String>(0))?
        .collect::<std::result::Result<_, _>>()?;

    match matches.as_slice() {
        [] => Err(Error::NotFound),
        [one] => parse(one),
        _ => Err(Error::Ambiguous {
            prefix: needle.to_string(),
            matches,
        }),
    }
}

pub(crate) fn parse(text: &str) -> Result<Uuid> {
    Uuid::parse_str(text).map_err(|e| Error::Corrupt(format!("id {text:?} ist keine UUID: {e}")))
}

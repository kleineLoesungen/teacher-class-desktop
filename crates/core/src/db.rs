//! Verbindung zur SQLite-Datei und Schemastand.
//!
//! Die Migrationen liegen als nummerierte `.sql`-Dateien in
//! `crates/core/migrations/` und werden über `include_str!` ins Binary
//! einkompiliert. Damit braucht das ausgelieferte Programm die Dateien nicht
//! mehr — eine Desktop-Anwendung hat kein Verzeichnis, aus dem sie sie lesen
//! könnte.
//!
//! **Eine Migration, die einmal veröffentlicht wurde, wird nie geändert.**
//! Sie ist auf fremden Rechnern bereits gelaufen. Änderungen kommen als neue
//! Datei mit der nächsten Nummer.

use std::path::Path;
use std::sync::LazyLock;

use rusqlite::Connection;
use rusqlite_migration::{M, Migrations};

use crate::error::{Error, Result};

static MIGRATIONS: LazyLock<Migrations<'static>> =
    LazyLock::new(|| Migrations::new(vec![M::up(include_str!("../migrations/0001_init.sql"))]));

/// Öffnet die Datei, legt sie bei Bedarf an und bringt das Schema auf Stand.
pub fn open(path: &Path) -> Result<Connection> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|source| Error::Io {
            path: parent.to_path_buf(),
            source,
        })?;
    }
    let mut conn = Connection::open(path)?;
    // WAL erlaubt Lesen während geschrieben wird. Für eine Anwendung, deren
    // Oberfläche und Kommandozeile gleichzeitig laufen können, ist das der
    // Unterschied zwischen "geht" und "database is locked".
    conn.pragma_update(None, "journal_mode", "WAL")?;
    prepare(&mut conn)?;
    Ok(conn)
}

/// Eine leere Datenbank im Arbeitsspeicher. Für Tests.
pub fn open_in_memory() -> Result<Connection> {
    let mut conn = Connection::open_in_memory()?;
    prepare(&mut conn)?;
    Ok(conn)
}

fn prepare(conn: &mut Connection) -> Result<()> {
    conn.pragma_update(None, "foreign_keys", "ON")?;
    conn.pragma_update(None, "synchronous", "NORMAL")?;
    // Statt sofort mit "database is locked" abzubrechen, fünf Sekunden warten.
    conn.busy_timeout(std::time::Duration::from_secs(5))?;
    MIGRATIONS.to_latest(conn)?;
    Ok(())
}

/// Prüft, dass die eingebauten Migrationen in sich stimmig sind.
///
/// Läuft als Test mit, damit ein Tippfehler im SQL beim `just check` auffällt
/// und nicht erst beim ersten Start auf einem fremden Rechner.
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn migrationen_sind_gueltig() {
        assert!(MIGRATIONS.validate().is_ok());
    }

    #[test]
    fn frische_datenbank_hat_das_schema() {
        let conn = open_in_memory().unwrap();
        let count: i64 = conn
            .query_row("SELECT count(*) FROM notes", [], |r| r.get(0))
            .unwrap();
        assert_eq!(count, 0);
    }
}

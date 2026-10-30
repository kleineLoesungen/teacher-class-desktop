//! Beispielressource — die Vorlage für jede eigene Ressource.
//!
//! Wer etwas Neues baut, kopiert dieses Modul und ändert die Namen. Was hier
//! steht, steht bewusst genau einmal:
//!
//! * **Die Prüfung der Eingabe gehört hierher**, nicht in die CLI und nicht in
//!   die Oberfläche. Sonst gibt es sie zweimal, leicht verschieden.
//! * **Kein `println!`, kein `clap`, kein `tauri`.** Dieses Modul weiß nicht,
//!   wer es aufruft. `crates/core/tests/layering.rs` besteht darauf.

use rusqlite::{Connection, OptionalExtension, Row};
use serde::{Deserialize, Serialize};
use time::OffsetDateTime;
use ts_rs::TS;
use uuid::Uuid;

use crate::error::{Error, Result, Validator};
use crate::timestamp;

/// Längster erlaubter Titel. Die Oberfläche zeigt den Rest sonst nicht mehr an.
pub const TITLE_MAX: usize = 120;
/// Längster erlaubter Text. Eine Notiz ist keine Datei.
pub const BODY_MAX: usize = 10_000;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
pub struct Note {
    pub id: Uuid,
    pub title: String,
    pub body: String,
    // In TypeScript ein RFC-3339-Text in UTC — so, wie serde ihn schreibt.
    #[serde(with = "crate::timestamp::serde_rfc3339")]
    #[ts(type = "string")]
    pub created_at: OffsetDateTime,
    #[serde(with = "crate::timestamp::serde_rfc3339")]
    #[ts(type = "string")]
    pub updated_at: OffsetDateTime,
}

/// Was von außen hereinkommt — ungeprüft.
///
/// Der Typ ist absichtlich ein anderer als [`Note`]: so lässt sich nichts
/// speichern, ohne vorher durch [`NoteInput::clean`] gegangen zu sein.
///
/// `Deserialize`, weil die Oberfläche genau diesen Typ über IPC schickt.
#[derive(Debug, Clone, Default, Deserialize, TS)]
pub struct NoteInput {
    pub title: String,
    pub body: String,
}

impl NoteInput {
    pub fn new(title: impl Into<String>, body: impl Into<String>) -> Self {
        Self {
            title: title.into(),
            body: body.into(),
        }
    }

    /// Räumt auf und prüft. Gibt die bereinigte Fassung zurück.
    ///
    /// Aufgeräumt wird zuerst, geprüft danach — ein Titel aus lauter
    /// Leerzeichen ist leer, nicht 20 Zeichen lang.
    pub fn clean(self) -> Result<NoteInput> {
        let title = self.title.trim().to_string();
        let body = self.body.trim_end().to_string();

        let mut v = Validator::new();
        v.require(!title.is_empty(), "title", "darf nicht leer sein");
        v.require(
            title.chars().count() <= TITLE_MAX,
            "title",
            format!("darf höchstens {TITLE_MAX} Zeichen lang sein"),
        );
        v.require(
            body.chars().count() <= BODY_MAX,
            "body",
            format!("darf höchstens {BODY_MAX} Zeichen lang sein"),
        );
        v.finish()?;

        Ok(NoteInput { title, body })
    }
}

/// Wonach die Liste sortiert wird.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Deserialize, TS)]
#[serde(rename_all = "lowercase")]
pub enum Order {
    /// Zuletzt geändert zuerst. Das, was man auf einem Schreibtisch erwartet.
    #[default]
    Recent,
    /// Alphabetisch nach Titel.
    Title,
}

pub fn list(conn: &Connection, order: Order) -> Result<Vec<Note>> {
    // Die Sortierung steht als festes SQL-Fragment da und kommt nie aus einer
    // Zeichenkette von außen — sonst wäre es eine SQL-Injektion.
    let sql = match order {
        Order::Recent => {
            "SELECT id, title, body, created_at, updated_at \
                          FROM notes ORDER BY updated_at DESC, id DESC"
        }
        Order::Title => {
            "SELECT id, title, body, created_at, updated_at \
                         FROM notes ORDER BY title COLLATE NOCASE ASC, id ASC"
        }
    };
    let mut stmt = conn.prepare(sql)?;
    let rows = stmt.query_map([], from_row)?;
    rows.collect::<std::result::Result<Vec<_>, _>>()?
        .into_iter()
        .collect()
}

pub fn get(conn: &Connection, id: Uuid) -> Result<Note> {
    conn.query_row(
        "SELECT id, title, body, created_at, updated_at FROM notes WHERE id = ?1",
        [id.to_string()],
        from_row,
    )
    .optional()?
    .ok_or(Error::NotFound)?
}

pub fn count(conn: &Connection) -> Result<i64> {
    Ok(conn.query_row("SELECT count(*) FROM notes", [], |r| r.get(0))?)
}

pub fn create(conn: &Connection, input: NoteInput) -> Result<Note> {
    let input = input.clean()?;
    let now = timestamp::now();
    // UUIDv7 trägt die Zeit im Präfix. Damit ist der Primärschlüssel
    // aufsteigend, was SQLite beim Einfügen entgegenkommt.
    let note = Note {
        id: Uuid::now_v7(),
        title: input.title,
        body: input.body,
        created_at: now,
        updated_at: now,
    };
    conn.execute(
        "INSERT INTO notes (id, title, body, created_at, updated_at) \
         VALUES (?1, ?2, ?3, ?4, ?5)",
        rusqlite::params![
            note.id.to_string(),
            note.title,
            note.body,
            timestamp::to_text(note.created_at),
            timestamp::to_text(note.updated_at),
        ],
    )?;
    tracing::debug!(id = %note.id, "Notiz angelegt");
    Ok(note)
}

pub fn update(conn: &Connection, id: Uuid, input: NoteInput) -> Result<Note> {
    let input = input.clean()?;
    let now = timestamp::now();
    let changed = conn.execute(
        "UPDATE notes SET title = ?2, body = ?3, updated_at = ?4 WHERE id = ?1",
        rusqlite::params![
            id.to_string(),
            input.title,
            input.body,
            timestamp::to_text(now),
        ],
    )?;
    if changed == 0 {
        return Err(Error::NotFound);
    }
    get(conn, id)
}

pub fn delete(conn: &Connection, id: Uuid) -> Result<()> {
    let changed = conn.execute("DELETE FROM notes WHERE id = ?1", [id.to_string()])?;
    if changed == 0 {
        return Err(Error::NotFound);
    }
    tracing::debug!(%id, "Notiz gelöscht");
    Ok(())
}

/// Eine Zeile in eine [`Note`] verwandeln.
///
/// Der Rückgabetyp ist doppelt verschachtelt, weil `query_map` einen
/// `rusqlite::Error` erwartet, ein unlesbarer Zeitstempel aber ein
/// [`Error::Corrupt`] ist. Die innere Schicht wird in [`list`] und [`get`]
/// wieder aufgelöst.
fn from_row(row: &Row<'_>) -> rusqlite::Result<Result<Note>> {
    let id: String = row.get(0)?;
    let created_at: String = row.get(3)?;
    let updated_at: String = row.get(4)?;
    Ok((|| {
        Ok(Note {
            id: Uuid::parse_str(&id)
                .map_err(|e| Error::Corrupt(format!("id {id:?} ist keine UUID: {e}")))?,
            title: row.get(1)?,
            body: row.get(2)?,
            created_at: timestamp::from_text(&created_at)?,
            updated_at: timestamp::from_text(&updated_at)?,
        })
    })())
}

/// Die Kurzform einer Kennung: die letzten acht Zeichen.
///
/// **Nicht die ersten.** Eine UUIDv7 beginnt mit dem Zeitstempel in
/// Millisekunden — zwei Notizen, die in derselben Sekunde entstehen, teilen
/// sich die ersten zwölf Zeichen. Zufällig ist erst das Ende.
pub fn short_id(id: Uuid) -> String {
    let full = id.to_string();
    full[full.len() - SHORT_ID_LEN..].to_string()
}

/// Länge der Kurzform. Acht Hexziffern sind 32 Bit Zufall — für eine
/// Notizsammlung auf einem Arbeitsplatzrechner reichlich.
pub const SHORT_ID_LEN: usize = 8;

/// Findet die Kennung zu einer vollständigen UUID oder einem eindeutigen Ende.
///
/// Eine UUID von Hand abzutippen ist zumutbar, aber lästig. Auf der
/// Kommandozeile genügt deshalb das Endstück, wie es `note list` anzeigt.
/// Ist es nicht eindeutig, gibt es [`Error::Ambiguous`] statt eines zufälligen
/// Treffers — „irgendeine davon" wäre bei `rm` die falsche Antwort.
pub fn resolve_id(conn: &Connection, needle: &str) -> Result<Uuid> {
    let needle = needle.trim();
    if let Ok(id) = Uuid::parse_str(needle) {
        // Existenz bestätigen, damit der Aufrufer sich auf die Kennung
        // verlassen kann.
        get(conn, id)?;
        return Ok(id);
    }
    if needle.is_empty() {
        return Err(Error::NotFound);
    }

    // Ein UUID-Stück besteht aus Hexziffern und Bindestrichen. Alles andere
    // wird abgewiesen, statt es für LIKE zu entschärfen — damit kann aus einem
    // "%" kein Joker werden, der versehentlich alles trifft.
    if !needle.chars().all(|c| c.is_ascii_hexdigit() || c == '-') {
        return Err(Error::NotFound);
    }

    // LIKE '%…' kann keinen Index benutzen. Bei einer Tabelle in der
    // Größenordnung einer Notizsammlung ist das belanglos; wer hier Millionen
    // Zeilen erwartet, legt eine Spalte mit dem Endstück an und indiziert sie.
    let mut stmt = conn.prepare("SELECT id FROM notes WHERE id LIKE ?1 ORDER BY id LIMIT 10")?;
    let matches: Vec<String> = stmt
        .query_map([format!("%{needle}")], |r| r.get::<_, String>(0))?
        .collect::<std::result::Result<_, _>>()?;

    match matches.as_slice() {
        [] => Err(Error::NotFound),
        [one] => Uuid::parse_str(one)
            .map_err(|e| Error::Corrupt(format!("id {one:?} ist keine UUID: {e}"))),
        _ => Err(Error::Ambiguous {
            prefix: needle.to_string(),
            matches,
        }),
    }
}

//! Klassen: ein Name (nur Buchstaben und Zahlen) und ein vierstelliges Jahr.
//!
//! Angezeigt wird sie als `7a (2025)`. Gespeichert werden Name und Jahr
//! getrennt. Die Prüfung steht hier, nicht in CLI oder Oberfläche.

use rusqlite::{Connection, OptionalExtension, Row};
use serde::{Deserialize, Serialize};
use time::OffsetDateTime;
use ts_rs::TS;
use uuid::Uuid;

use crate::error::{Error, Result, Validator};
use crate::{ids, timestamp};

/// Längster Name. Der Excel-Blattname `Name (Jahr)` darf höchstens 31 Zeichen
/// haben: 24 + `" ("` + vier Ziffern + `")"` = 31.
pub const NAME_MAX: usize = 24;
pub const YEAR_MIN: i32 = 1000;
pub const YEAR_MAX: i32 = 9999;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
pub struct Class {
    pub id: Uuid,
    pub name: String,
    pub year: i32,
    /// Wie viele Schüler in der Klasse sind.
    pub student_count: i64,
    #[serde(with = "crate::timestamp::serde_rfc3339")]
    #[ts(type = "string")]
    pub created_at: OffsetDateTime,
}

impl Class {
    /// `7a (2025)`
    pub fn label(&self) -> String {
        label(&self.name, self.year)
    }
}

pub fn label(name: &str, year: i32) -> String {
    format!("{name} ({year})")
}

/// Was von außen hereinkommt — ungeprüft.
#[derive(Debug, Clone, Default, Deserialize, TS)]
pub struct ClassInput {
    pub name: String,
    pub year: i32,
}

impl ClassInput {
    pub fn new(name: impl Into<String>, year: i32) -> Self {
        Self {
            name: name.into(),
            year,
        }
    }

    pub fn clean(self) -> Result<ClassInput> {
        let name = self.name.trim().to_string();

        let mut v = Validator::new();
        v.require(!name.is_empty(), "name", "darf nicht leer sein");
        v.require(
            name.chars().all(char::is_alphanumeric),
            "name",
            "darf nur Buchstaben und Zahlen enthalten",
        );
        v.require(
            name.chars().count() <= NAME_MAX,
            "name",
            format!("darf höchstens {NAME_MAX} Zeichen lang sein"),
        );
        v.require(
            (YEAR_MIN..=YEAR_MAX).contains(&self.year),
            "year",
            "muss ein vierstelliges Jahr sein",
        );
        v.finish()?;

        Ok(ClassInput {
            name,
            year: self.year,
        })
    }
}

const SELECT: &str = "SELECT c.id, c.name, c.year, c.created_at, \
     (SELECT count(*) FROM students s WHERE s.class_id = c.id) FROM classes c";

/// Nach Jahr absteigend (neuester Jahrgang zuerst), dann nach Name.
pub fn list(conn: &Connection) -> Result<Vec<Class>> {
    let mut stmt = conn.prepare(&format!(
        "{SELECT} ORDER BY c.year DESC, c.name COLLATE NOCASE ASC"
    ))?;
    let rows = stmt.query_map([], from_row)?;
    rows.collect::<std::result::Result<Vec<_>, _>>()?
        .into_iter()
        .collect()
}

pub fn get(conn: &Connection, id: Uuid) -> Result<Class> {
    conn.query_row(
        &format!("{SELECT} WHERE c.id = ?1"),
        [id.to_string()],
        from_row,
    )
    .optional()?
    .ok_or(Error::NotFound)?
}

pub fn create(conn: &Connection, input: ClassInput) -> Result<Class> {
    let input = input.clean()?;
    ensure_unique(conn, &input, None)?;
    let id = Uuid::now_v7();
    conn.execute(
        "INSERT INTO classes (id, name, year, created_at) VALUES (?1, ?2, ?3, ?4)",
        rusqlite::params![
            id.to_string(),
            input.name,
            input.year,
            timestamp::to_text(timestamp::now()),
        ],
    )?;
    tracing::debug!(%id, "Klasse angelegt");
    get(conn, id)
}

pub fn update(conn: &Connection, id: Uuid, input: ClassInput) -> Result<Class> {
    let input = input.clean()?;
    get(conn, id)?;
    ensure_unique(conn, &input, Some(id))?;
    conn.execute(
        "UPDATE classes SET name = ?2, year = ?3 WHERE id = ?1",
        rusqlite::params![id.to_string(), input.name, input.year],
    )?;
    get(conn, id)
}

/// Löscht die Klasse samt ihren Schülern.
pub fn delete(conn: &Connection, id: Uuid) -> Result<()> {
    let changed = conn.execute("DELETE FROM classes WHERE id = ?1", [id.to_string()])?;
    if changed == 0 {
        return Err(Error::NotFound);
    }
    tracing::debug!(%id, "Klasse gelöscht");
    Ok(())
}

pub fn resolve_id(conn: &Connection, needle: &str) -> Result<Uuid> {
    ids::resolve(conn, ids::Table::Classes, needle)
}

/// Dieselbe Klasse (Name ohne Beachtung der Groß-/Kleinschreibung, Jahr) gibt
/// es nur einmal — sonst wären die Excel-Blattnamen doppelt.
fn ensure_unique(conn: &Connection, input: &ClassInput, own: Option<Uuid>) -> Result<()> {
    let other: Option<String> = conn
        .query_row(
            "SELECT id FROM classes WHERE name = ?1 COLLATE NOCASE AND year = ?2 \
             AND id != ?3",
            rusqlite::params![
                input.name,
                input.year,
                own.map(|i| i.to_string()).unwrap_or_default()
            ],
            |r| r.get(0),
        )
        .optional()?;
    let mut v = Validator::new();
    v.require(other.is_none(), "name", "Diese Klasse gibt es schon");
    v.finish()
}

fn from_row(row: &Row<'_>) -> rusqlite::Result<Result<Class>> {
    let id: String = row.get(0)?;
    let created_at: String = row.get(3)?;
    Ok((|| {
        Ok(Class {
            id: ids::parse(&id)?,
            name: row.get(1)?,
            year: row.get(2)?,
            created_at: timestamp::from_text(&created_at)?,
            student_count: row.get(4)?,
        })
    })())
}

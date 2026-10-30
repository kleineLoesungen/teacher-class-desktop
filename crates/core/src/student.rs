//! Schüler einer Klasse. Schreibweise: `Nachname, Vorname`.
//!
//! Mehrere Schüler werden als Text mit einer Zeile je Schüler angelegt — das
//! ist zugleich der Weg für einen einzelnen. Alle Zeilen werden zuerst geprüft
//! und dann in einer Transaktion gespeichert: entweder alle oder keine.

use rusqlite::{Connection, OptionalExtension, Row};
use serde::{Deserialize, Serialize};
use ts_rs::TS;
use uuid::Uuid;

use crate::error::{Error, Result, Validator};
use crate::{class, ids, timestamp};

/// Längster Nach- bzw. Vorname.
pub const NAME_MAX: usize = 60;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
pub struct Student {
    pub id: Uuid,
    pub class_id: Uuid,
    pub last_name: String,
    pub first_name: String,
}

impl Student {
    /// `Nachname, Vorname`
    pub fn label(&self) -> String {
        format!("{}, {}", self.last_name, self.first_name)
    }
}

/// Ein Schüler, ungeprüft.
#[derive(Debug, Clone, Default, Deserialize, TS)]
pub struct StudentInput {
    pub last_name: String,
    pub first_name: String,
}

impl StudentInput {
    pub fn clean(self) -> Result<StudentInput> {
        let last_name = self.last_name.trim().to_string();
        let first_name = self.first_name.trim().to_string();
        let mut v = Validator::new();
        check_name(&mut v, "last_name", &last_name);
        check_name(&mut v, "first_name", &first_name);
        v.finish()?;
        Ok(StudentInput {
            last_name,
            first_name,
        })
    }
}

fn check_name(v: &mut Validator, field: &'static str, value: &str) {
    v.require(!value.is_empty(), field, "darf nicht leer sein");
    v.require(
        value.chars().count() <= NAME_MAX,
        field,
        format!("darf höchstens {NAME_MAX} Zeichen lang sein"),
    );
}

/// Zerlegt eine Zeile `Nachname, Vorname`.
fn parse_line(line: &str) -> std::result::Result<StudentInput, String> {
    let mut parts = line.split(',');
    let (Some(last), Some(first), None) = (parts.next(), parts.next(), parts.next()) else {
        return Err("erwartet „Nachname, Vorname“ mit genau einem Komma".to_string());
    };
    let (last, first) = (last.trim(), first.trim());
    if last.is_empty() || first.is_empty() {
        return Err("Nachname und Vorname dürfen nicht leer sein".to_string());
    }
    if last.chars().count() > NAME_MAX || first.chars().count() > NAME_MAX {
        return Err(format!(
            "Namen dürfen höchstens {NAME_MAX} Zeichen lang sein"
        ));
    }
    Ok(StudentInput {
        last_name: last.to_string(),
        first_name: first.to_string(),
    })
}

/// Legt Schüler aus Text an, eine Zeile je Schüler. Leere Zeilen zählen nicht.
///
/// Fehler tragen die Zeilennummer (ab 1) und beziehen sich auf das Feld
/// `students`. Bei einem Fehler wird nichts gespeichert.
pub fn add_many(conn: &Connection, class_id: Uuid, text: &str) -> Result<Vec<Student>> {
    class::get(conn, class_id)?;

    let mut v = Validator::new();
    let mut inputs = Vec::new();
    for (i, line) in text.lines().enumerate() {
        if line.trim().is_empty() {
            continue;
        }
        match parse_line(line) {
            Ok(input) => inputs.push(input),
            Err(msg) => v.add("students", format!("Zeile {}: {msg}", i + 1)),
        }
    }
    if inputs.is_empty() {
        v.add("students", "mindestens ein Schüler nötig");
    }
    v.finish()?;

    let tx = conn.unchecked_transaction()?;
    let mut created = Vec::with_capacity(inputs.len());
    for input in inputs {
        let student = Student {
            id: Uuid::now_v7(),
            class_id,
            last_name: input.last_name,
            first_name: input.first_name,
        };
        tx.execute(
            "INSERT INTO students (id, class_id, last_name, first_name, created_at) \
             VALUES (?1, ?2, ?3, ?4, ?5)",
            rusqlite::params![
                student.id.to_string(),
                class_id.to_string(),
                student.last_name,
                student.first_name,
                timestamp::to_text(timestamp::now()),
            ],
        )?;
        created.push(student);
    }
    tx.commit()?;
    Ok(created)
}

/// Alphabetisch nach Nachname, dann Vorname.
pub fn list(conn: &Connection, class_id: Uuid) -> Result<Vec<Student>> {
    class::get(conn, class_id)?;
    let mut stmt = conn.prepare(
        "SELECT id, class_id, last_name, first_name FROM students WHERE class_id = ?1 \
         ORDER BY last_name COLLATE NOCASE, first_name COLLATE NOCASE, id",
    )?;
    let rows = stmt.query_map([class_id.to_string()], from_row)?;
    rows.collect::<std::result::Result<Vec<_>, _>>()?
        .into_iter()
        .collect()
}

pub fn get(conn: &Connection, id: Uuid) -> Result<Student> {
    conn.query_row(
        "SELECT id, class_id, last_name, first_name FROM students WHERE id = ?1",
        [id.to_string()],
        from_row,
    )
    .optional()?
    .ok_or(Error::NotFound)?
}

pub fn update(conn: &Connection, id: Uuid, input: StudentInput) -> Result<Student> {
    let input = input.clean()?;
    let changed = conn.execute(
        "UPDATE students SET last_name = ?2, first_name = ?3 WHERE id = ?1",
        rusqlite::params![id.to_string(), input.last_name, input.first_name],
    )?;
    if changed == 0 {
        return Err(Error::NotFound);
    }
    get(conn, id)
}

pub fn delete(conn: &Connection, id: Uuid) -> Result<()> {
    let changed = conn.execute("DELETE FROM students WHERE id = ?1", [id.to_string()])?;
    if changed == 0 {
        return Err(Error::NotFound);
    }
    Ok(())
}

pub fn resolve_id(conn: &Connection, needle: &str) -> Result<Uuid> {
    ids::resolve(conn, ids::Table::Students, needle)
}

fn from_row(row: &Row<'_>) -> rusqlite::Result<Result<Student>> {
    let id: String = row.get(0)?;
    let class_id: String = row.get(1)?;
    Ok((|| {
        Ok(Student {
            id: ids::parse(&id)?,
            class_id: ids::parse(&class_id)?,
            last_name: row.get(2)?,
            first_name: row.get(3)?,
        })
    })())
}

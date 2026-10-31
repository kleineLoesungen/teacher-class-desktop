//! Bewertung: erreichte Punkte und Begründungen je Schüler und Prüfkriterium.
//!
//! * Ein Eintrag gehört zu (Prüfung, Schüler, Gruppe, Punkt). Gruppe und Punkt
//!   sind **Stellen** (nullbasiert), keine Kennungen — siehe Migration 0004.
//! * Punkte sind ganze oder halbe Zahlen von 0 bis zur Höchstpunktzahl des
//!   Punktes. „Nicht eingetragen“ (`None`) ist etwas anderes als 0.
//! * Eine Begründung gibt es nur, wenn die Prüfung `with_notes` trägt.
//! * Gruppensummen, Gesamtpunkte und Note **berechnet diese Datei**; die
//!   Oberfläche rechnet nicht nach. Die Note bleibt leer, solange für den
//!   Schüler kein einziger Punkt eingetragen ist (wie im Excel-Export).
//!
//! Wird die Prüfung geändert, bleiben die Einträge, soweit sie noch passen
//! ([`prune`]).

use std::collections::HashMap;

use rusqlite::{Connection, params};
use serde::{Deserialize, Serialize};
use ts_rs::TS;
use uuid::Uuid;

use crate::error::{Error, Result, Validator};
use crate::exam::{self, Exam, from_half, to_half};
use crate::student::{self, Student};

/// Längste Begründung.
pub const NOTE_MAX: usize = 2000;

// --- Ausgabe ---------------------------------------------------------------

/// Der Eintrag zu einem Prüfkriterium.
#[derive(Debug, Clone, PartialEq, Serialize, TS)]
pub struct Score {
    /// `None`: noch nicht bewertet.
    pub points: Option<f64>,
    /// Begründung, leer = keine.
    pub note: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, TS)]
pub struct GroupResult {
    /// Je Punkt der Gruppe, in der Reihenfolge des Schemas.
    pub scores: Vec<Score>,
    /// Summe der eingetragenen Punkte. Berechnet.
    pub points: f64,
}

/// Die Bewertung eines Schülers.
#[derive(Debug, Clone, PartialEq, Serialize, TS)]
pub struct StudentResult {
    pub student: Student,
    /// Je Gruppe des Schemas.
    pub groups: Vec<GroupResult>,
    /// Gesamtpunkte. Berechnet.
    pub total: f64,
    /// Note aus der Bewertungsmatrix; `None`, solange nichts eingetragen ist.
    pub grade: Option<String>,
}

// --- Eingabe ---------------------------------------------------------------

/// Ein Eintrag, ungeprüft. Ersetzt den bisherigen Eintrag zu diesem Punkt.
#[derive(Debug, Clone, Default, Deserialize, TS)]
pub struct ScoreInput {
    pub student_id: Uuid,
    /// Stelle der Gruppe im Schema, nullbasiert.
    pub group: u32,
    /// Stelle des Punktes in der Gruppe, nullbasiert.
    pub item: u32,
    /// `None` löscht die Punkte.
    pub points: Option<f64>,
    #[serde(default)]
    pub note: String,
}

// --- Lesen -----------------------------------------------------------------

type Key = (String, usize, usize);
type Stored = (Option<i64>, String);

fn load(conn: &Connection, exam_id: Uuid) -> Result<HashMap<Key, Stored>> {
    let mut stmt = conn.prepare(
        "SELECT student_id, group_pos, item_pos, points_half, note \
         FROM exam_scores WHERE exam_id = ?1",
    )?;
    let rows = stmt.query_map([exam_id.to_string()], |r| {
        Ok((
            (
                r.get::<_, String>(0)?,
                r.get::<_, i64>(1)?,
                r.get::<_, i64>(2)?,
            ),
            (r.get::<_, Option<i64>>(3)?, r.get::<_, String>(4)?),
        ))
    })?;
    let mut map = HashMap::new();
    for row in rows {
        let ((student, group, item), stored) = row?;
        map.insert((student, group as usize, item as usize), stored);
    }
    Ok(map)
}

fn compute(exam: &Exam, student: Student, stored: &HashMap<Key, Stored>) -> StudentResult {
    let sid = student.id.to_string();
    let mut total: i64 = 0;
    let mut any = false;
    let mut groups = Vec::new();
    for (gi, group) in exam.groups.iter().enumerate() {
        let mut sum: i64 = 0;
        let mut scores = Vec::new();
        for ii in 0..group.items.len() {
            let (half, note) = stored
                .get(&(sid.clone(), gi, ii))
                .cloned()
                .unwrap_or((None, String::new()));
            if let Some(h) = half {
                sum += h;
                any = true;
            }
            scores.push(Score {
                points: half.map(from_half),
                note,
            });
        }
        total += sum;
        groups.push(GroupResult {
            scores,
            points: from_half(sum),
        });
    }
    let total = from_half(total);
    // Matrix aufsteigend: die höchste Note, deren „ab“ erreicht ist.
    let grade = any
        .then(|| {
            exam.grades
                .iter()
                .rev()
                .find(|g| g.from_points <= total)
                .map(|g| g.name.clone())
        })
        .flatten();
    StudentResult {
        student,
        groups,
        total,
        grade,
    }
}

/// Die Bewertung aller Schüler der Klasse `class_id` in der Prüfung. Die
/// Klasse muss zur Prüfung gehören.
pub fn results(conn: &Connection, exam_id: Uuid, class_id: Uuid) -> Result<Vec<StudentResult>> {
    let exam = exam::get(conn, exam_id)?;
    if !exam.class_ids.contains(&class_id) {
        return Err(Error::NotFound);
    }
    let stored = load(conn, exam_id)?;
    Ok(student::list(conn, class_id)?
        .into_iter()
        .map(|s| compute(&exam, s, &stored))
        .collect())
}

// --- Schreiben -------------------------------------------------------------

/// Trägt Punkte und Begründung zu einem Prüfkriterium ein und gibt die
/// Bewertung des Schülers neu berechnet zurück. Sind Punkte und Begründung
/// leer, verschwindet der Eintrag.
pub fn set(conn: &Connection, exam_id: Uuid, input: ScoreInput) -> Result<StudentResult> {
    let exam = exam::get(conn, exam_id)?;
    let student = student::get(conn, input.student_id)?;
    if !exam.class_ids.contains(&student.class_id) {
        return Err(Error::NotFound);
    }

    let mut v = Validator::new();
    let item = exam
        .groups
        .get(input.group as usize)
        .and_then(|g| g.items.get(input.item as usize));
    v.require(item.is_some(), "item", "dieses Prüfkriterium gibt es nicht");

    let mut half = None;
    if let (Some(points), Some(item)) = (input.points, item) {
        match to_half(points) {
            Some(h) if h >= 0 && points <= item.max_points => half = Some(h),
            _ => v.add(
                "points",
                format!(
                    "Punkte müssen zwischen 0 und {} liegen, in ganzen oder halben Punkten",
                    item.max_points
                ),
            ),
        }
    }

    let note = input.note.trim().to_string();
    v.require(
        note.chars().count() <= NOTE_MAX,
        "note",
        format!("darf höchstens {NOTE_MAX} Zeichen lang sein"),
    );
    v.require(
        note.is_empty() || exam.with_notes,
        "note",
        "für diese Prüfung sind keine Begründungen vorgesehen",
    );
    v.finish()?;

    let sid = student.id.to_string();
    if half.is_none() && note.is_empty() {
        conn.execute(
            "DELETE FROM exam_scores \
             WHERE exam_id = ?1 AND student_id = ?2 AND group_pos = ?3 AND item_pos = ?4",
            params![exam_id.to_string(), sid, input.group, input.item],
        )?;
    } else {
        conn.execute(
            "INSERT INTO exam_scores (exam_id, student_id, group_pos, item_pos, points_half, note) \
             VALUES (?1, ?2, ?3, ?4, ?5, ?6) \
             ON CONFLICT (exam_id, student_id, group_pos, item_pos) \
             DO UPDATE SET points_half = excluded.points_half, note = excluded.note",
            params![
                exam_id.to_string(),
                sid,
                input.group,
                input.item,
                half,
                note
            ],
        )?;
    }
    let stored = load(conn, exam_id)?;
    Ok(compute(&exam, student, &stored))
}

/// Räumt nach einer Änderung der Prüfung auf: Einträge zu Punkten, die es nicht
/// mehr gibt, zu Schülern aus entfernten Klassen und Begründungen einer
/// Prüfung ohne Begründungen entfallen; Punkte über der neuen Höchstpunktzahl
/// werden zurückgenommen (die Begründung bleibt).
pub(crate) fn prune(conn: &Connection, exam_id: Uuid) -> Result<()> {
    let id = exam_id.to_string();
    conn.execute(
        "DELETE FROM exam_scores WHERE exam_id = ?1 AND student_id NOT IN ( \
             SELECT s.id FROM students s JOIN exam_classes e ON e.class_id = s.class_id \
             WHERE e.exam_id = ?1)",
        [&id],
    )?;
    conn.execute(
        "DELETE FROM exam_scores WHERE exam_id = ?1 AND NOT EXISTS ( \
             SELECT 1 FROM exam_groups g JOIN exam_items i ON i.group_id = g.id \
             WHERE g.exam_id = ?1 AND g.position = group_pos AND i.position = item_pos)",
        [&id],
    )?;
    conn.execute(
        "UPDATE exam_scores SET points_half = NULL WHERE exam_id = ?1 AND points_half > ( \
             SELECT i.max_half FROM exam_groups g JOIN exam_items i ON i.group_id = g.id \
             WHERE g.exam_id = ?1 AND g.position = group_pos AND i.position = item_pos)",
        [&id],
    )?;
    conn.execute(
        "UPDATE exam_scores SET note = '' WHERE exam_id = ?1 AND \
             (SELECT with_notes FROM exams WHERE id = ?1) = 0",
        [&id],
    )?;
    conn.execute(
        "DELETE FROM exam_scores WHERE exam_id = ?1 AND points_half IS NULL AND note = ''",
        [&id],
    )?;
    Ok(())
}

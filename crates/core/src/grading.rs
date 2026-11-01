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
//! * Zusätzlich gibt es je Schüler einen **Abschluss-Satz** ([`set_remark`]),
//!   unabhängig von `with_notes`.
//! * [`suggest`] schlägt beim Tippen Texte vor, die bei anderen Schülern
//!   derselben Prüfung schon stehen.
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
    /// Abschluss-Satz, leer = keiner.
    pub remark: String,
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

/// Der Abschluss-Satz eines Schülers, ungeprüft. Leer löscht ihn.
#[derive(Debug, Clone, Default, Deserialize, TS)]
pub struct RemarkInput {
    pub student_id: Uuid,
    #[serde(default)]
    pub text: String,
}

/// Wofür ein Vorschlag gedacht ist.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
pub enum SuggestKind {
    /// Begründung zu einem Prüfkriterium.
    Note,
    /// Abschluss-Satz.
    Remark,
}

/// Anfrage nach Textvorschlägen.
#[derive(Debug, Clone, Deserialize, TS)]
pub struct SuggestInput {
    pub kind: SuggestKind,
    /// Das bisher Getippte; Vorschläge enthalten jedes Wort daraus.
    #[serde(default)]
    pub query: String,
    /// Dieser Schüler wird nicht berücksichtigt (sein eigener Text).
    #[serde(default)]
    pub exclude_student_id: Option<Uuid>,
}

/// Höchstzahl der Vorschläge.
pub const SUGGEST_MAX: usize = 8;

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

fn load_remarks(conn: &Connection, exam_id: Uuid) -> Result<HashMap<String, String>> {
    let mut stmt = conn.prepare("SELECT student_id, text FROM exam_remarks WHERE exam_id = ?1")?;
    let rows = stmt.query_map([exam_id.to_string()], |r| {
        Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?))
    })?;
    let mut map = HashMap::new();
    for row in rows {
        let (student, text) = row?;
        map.insert(student, text);
    }
    Ok(map)
}

fn compute(
    exam: &Exam,
    student: Student,
    stored: &HashMap<Key, Stored>,
    remarks: &HashMap<String, String>,
) -> StudentResult {
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
        remark: remarks.get(&sid).cloned().unwrap_or_default(),
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
    let remarks = load_remarks(conn, exam_id)?;
    Ok(student::list(conn, class_id)?
        .into_iter()
        .map(|s| compute(&exam, s, &stored, &remarks))
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
    let remarks = load_remarks(conn, exam_id)?;
    Ok(compute(&exam, student, &stored, &remarks))
}

/// Setzt den Abschluss-Satz eines Schülers und gibt dessen Bewertung zurück.
/// Ein leerer Text löscht ihn.
pub fn set_remark(conn: &Connection, exam_id: Uuid, input: RemarkInput) -> Result<StudentResult> {
    let exam = exam::get(conn, exam_id)?;
    let student = student::get(conn, input.student_id)?;
    if !exam.class_ids.contains(&student.class_id) {
        return Err(Error::NotFound);
    }
    let text = input.text.trim().to_string();
    let mut v = Validator::new();
    v.require(
        text.chars().count() <= NOTE_MAX,
        "remark",
        format!("darf höchstens {NOTE_MAX} Zeichen lang sein"),
    );
    v.finish()?;

    let sid = student.id.to_string();
    if text.is_empty() {
        conn.execute(
            "DELETE FROM exam_remarks WHERE exam_id = ?1 AND student_id = ?2",
            params![exam_id.to_string(), sid],
        )?;
    } else {
        conn.execute(
            "INSERT INTO exam_remarks (exam_id, student_id, text) VALUES (?1, ?2, ?3) \
             ON CONFLICT (exam_id, student_id) DO UPDATE SET text = excluded.text",
            params![exam_id.to_string(), sid, text],
        )?;
    }
    let stored = load(conn, exam_id)?;
    let remarks = load_remarks(conn, exam_id)?;
    Ok(compute(&exam, student, &stored, &remarks))
}

// --- Vorschläge ------------------------------------------------------------

/// Texte, die bei anderen Schülern derselben Prüfung schon stehen und alle
/// Wörter der Eingabe enthalten (ohne Beachtung der Groß-/Kleinschreibung).
/// Gleiche Texte erscheinen einmal, die häufigsten zuerst, dann alphabetisch.
/// Ein Text, der genau so schon getippt ist, wird nicht vorgeschlagen. Ohne
/// Eingabe kommen die häufigsten Texte.
pub fn suggest(conn: &Connection, exam_id: Uuid, input: SuggestInput) -> Result<Vec<String>> {
    exam::get(conn, exam_id)?;
    let sql = match input.kind {
        SuggestKind::Note => {
            "SELECT note, student_id FROM exam_scores WHERE exam_id = ?1 AND note <> ''"
        }
        SuggestKind::Remark => "SELECT text, student_id FROM exam_remarks WHERE exam_id = ?1",
    };
    let excluded = input.exclude_student_id.map(|u| u.to_string());
    let mut stmt = conn.prepare(sql)?;
    let rows = stmt.query_map([exam_id.to_string()], |r| {
        Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?))
    })?;

    let typed = input.query.trim().to_lowercase();
    let words: Vec<&str> = typed.split_whitespace().collect();
    let mut counts: HashMap<String, usize> = HashMap::new();
    for row in rows {
        let (text, sid) = row?;
        if excluded.as_deref() == Some(sid.as_str()) {
            continue;
        }
        let lower = text.to_lowercase();
        if lower == typed || !words.iter().all(|w| lower.contains(w)) {
            continue;
        }
        *counts.entry(text).or_default() += 1;
    }
    let mut list: Vec<(String, usize)> = counts.into_iter().collect();
    list.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
    Ok(list.into_iter().take(SUGGEST_MAX).map(|(t, _)| t).collect())
}

// --- Notenschlüssel --------------------------------------------------------

/// Wie viele Schüler eine Note erreicht haben.
#[derive(Debug, Clone, PartialEq, Serialize, TS)]
pub struct GradeCount {
    pub grade: String,
    pub count: u32,
}

/// Der Notenschlüssel einer Klasse: wie oft jede Note vorkommt, und der
/// Durchschnitt. Berechnet von `core`, die Oberfläche rechnet nicht nach.
#[derive(Debug, Clone, PartialEq, Serialize, TS)]
pub struct GradeKey {
    /// Je Note der Bewertungsmatrix, in deren Reihenfolge (auch mit 0).
    pub counts: Vec<GradeCount>,
    /// Schüler mit einer Note, also mit mindestens einem eingetragenen Punkt.
    pub graded: u32,
    /// Durchschnitt der Noten auf zwei Nachkommastellen. `None`, wenn noch
    /// niemand eine Note hat oder ein Notenname keine Zahl ist (z. B. „A“).
    pub average: Option<f64>,
}

/// Der Notenschlüssel aus bereits berechneten Bewertungen.
pub fn grade_key_of(exam: &Exam, results: &[StudentResult]) -> GradeKey {
    let counts: Vec<GradeCount> = exam
        .grades
        .iter()
        .map(|g| GradeCount {
            grade: g.name.clone(),
            count: results
                .iter()
                .filter(|r| r.grade.as_deref() == Some(g.name.as_str()))
                .count() as u32,
        })
        .collect();
    let graded: u32 = counts.iter().map(|c| c.count).sum();
    let mut sum = 0.0;
    let mut numeric = true;
    for c in &counts {
        match c.grade.trim().replace(',', ".").parse::<f64>() {
            Ok(n) => sum += n * f64::from(c.count),
            Err(_) if c.count > 0 => numeric = false,
            Err(_) => {}
        }
    }
    let average =
        (graded > 0 && numeric).then(|| (sum / f64::from(graded) * 100.0).round() / 100.0);
    GradeKey {
        counts,
        graded,
        average,
    }
}

/// Der Notenschlüssel der Klasse `class_id` in der Prüfung.
pub fn grade_key(conn: &Connection, exam_id: Uuid, class_id: Uuid) -> Result<GradeKey> {
    let exam = exam::get(conn, exam_id)?;
    let results = results(conn, exam_id, class_id)?;
    Ok(grade_key_of(&exam, &results))
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

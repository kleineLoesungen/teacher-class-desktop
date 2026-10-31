//! Prüfungen: Prüfungsschema und Bewertungsmatrix.
//!
//! * **Schema:** Gruppen mit Namen; darin Punkte (Aufgaben) mit Namen und
//!   größter Punktzahl. Die Höchstpunktzahl einer Gruppe und der Prüfung
//!   werden berechnet, nicht eingegeben.
//! * **Bewertungsmatrix:** Noten mit einem „ab“ (Punkte). Das „bis“ wird
//!   berechnet: eine halbe Punktstufe unter dem „ab“ der nächsthöheren Note,
//!   bei der höchsten Note die Gesamtpunktzahl. Dadurch gibt es weder Lücken
//!   noch Überschneidungen. Die niedrigste Note beginnt bei 0.
//!
//! Punkte sind ganze oder halbe Zahlen. Gespeichert werden Halbpunkte als
//! ganze Zahl (2,5 → 5), damit nichts rundet.
//!
//! Jeder Punkt (jedes Prüfkriterium) kann eine Beschreibung tragen. Ist
//! `with_notes` gesetzt, wird zu jedem Punkt auch eine Begründung gepflegt
//! (siehe [`crate::grading`]).
//!
//! Eine Prüfung wird immer als Ganzes angelegt oder ersetzt (`ExamInput`);
//! es gibt keine einzelnen Änderungen an Gruppen oder Noten.

use rusqlite::{Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use time::OffsetDateTime;
use ts_rs::TS;
use uuid::Uuid;

use crate::error::{Error, Result, Validator};
use crate::{class, ids, timestamp};

pub const NAME_MAX: usize = 100;
/// Größte Punktzahl eines einzelnen Punktes.
pub const POINTS_MAX: f64 = 1000.0;
/// Längste Beschreibung eines Prüfkriteriums.
pub const DESCRIPTION_MAX: usize = 500;

// --- Ausgabe ---------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Serialize, TS)]
pub struct Item {
    pub name: String,
    /// Optional, leer = keine.
    pub description: String,
    pub max_points: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, TS)]
pub struct Group {
    pub name: String,
    pub items: Vec<Item>,
    /// Summe der Punkte der Gruppe. Berechnet.
    pub max_points: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, TS)]
pub struct Grade {
    pub name: String,
    pub from_points: f64,
    /// Berechnet, siehe Moduldoku.
    pub to_points: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, TS)]
pub struct Exam {
    pub id: Uuid,
    pub name: String,
    pub class_ids: Vec<Uuid>,
    pub groups: Vec<Group>,
    /// Aufsteigend nach „ab“: die schlechteste Note zuerst.
    pub grades: Vec<Grade>,
    /// Gesamtpunktzahl. Berechnet.
    pub max_points: f64,
    /// Ob zu jedem Prüfkriterium eine Begründung gepflegt wird.
    pub with_notes: bool,
    #[serde(with = "crate::timestamp::serde_rfc3339")]
    #[ts(type = "string")]
    pub created_at: OffsetDateTime,
}

// --- Eingabe ---------------------------------------------------------------

#[derive(Debug, Clone, Default, Deserialize, TS)]
pub struct ItemInput {
    pub name: String,
    #[serde(default)]
    pub description: String,
    pub max_points: f64,
}

#[derive(Debug, Clone, Default, Deserialize, TS)]
pub struct GroupInput {
    pub name: String,
    pub items: Vec<ItemInput>,
}

#[derive(Debug, Clone, Default, Deserialize, TS)]
pub struct GradeInput {
    pub name: String,
    pub from_points: f64,
}

/// Was von außen hereinkommt — ungeprüft.
#[derive(Debug, Clone, Default, Deserialize, TS)]
pub struct ExamInput {
    pub name: String,
    pub class_ids: Vec<Uuid>,
    pub groups: Vec<GroupInput>,
    pub grades: Vec<GradeInput>,
    /// Ob zu jedem Prüfkriterium eine Begründung gepflegt wird.
    #[serde(default)]
    pub with_notes: bool,
}

/// Halbpunkte als ganze Zahl, falls `points` ein Vielfaches von 0,5 ist.
pub(crate) fn to_half(points: f64) -> Option<i64> {
    if !points.is_finite() {
        return None;
    }
    let doubled = points * 2.0;
    (doubled.fract() == 0.0 && doubled.abs() < 1e9).then_some(doubled as i64)
}

pub(crate) fn from_half(half: i64) -> f64 {
    half as f64 / 2.0
}

/// Geprüft und in Halbpunkten.
struct Clean {
    name: String,
    class_ids: Vec<Uuid>,
    with_notes: bool,
    groups: Vec<(String, Vec<CleanItem>)>,
    /// Aufsteigend nach `from_half`.
    grades: Vec<(String, i64)>,
}

struct CleanItem {
    name: String,
    description: String,
    half: i64,
}

impl ExamInput {
    fn clean(self) -> Result<Clean> {
        let name = self.name.trim().to_string();
        let mut v = Validator::new();
        v.require(!name.is_empty(), "name", "darf nicht leer sein");
        v.require(
            name.chars().count() <= NAME_MAX,
            "name",
            format!("darf höchstens {NAME_MAX} Zeichen lang sein"),
        );

        let mut class_ids = self.class_ids;
        class_ids.dedup();
        let mut seen = std::collections::HashSet::new();
        class_ids.retain(|id| seen.insert(*id));
        v.require(
            !class_ids.is_empty(),
            "classes",
            "mindestens eine Klasse wählen",
        );

        v.require(
            !self.groups.is_empty(),
            "groups",
            "mindestens eine Gruppe nötig",
        );
        let mut groups = Vec::new();
        let mut total: i64 = 0;
        let errors_before = v.count();
        for (gi, group) in self.groups.into_iter().enumerate() {
            let gname = group.name.trim().to_string();
            let g = gi + 1;
            v.require(
                !gname.is_empty(),
                "groups",
                format!("Gruppe {g}: Name darf nicht leer sein"),
            );
            v.require(
                !group.items.is_empty(),
                "groups",
                format!("Gruppe {g}: mindestens ein Punkt nötig"),
            );
            let mut items = Vec::new();
            for (ii, item) in group.items.into_iter().enumerate() {
                let iname = item.name.trim().to_string();
                let i = ii + 1;
                v.require(
                    !iname.is_empty(),
                    "groups",
                    format!("Gruppe {g}, Punkt {i}: Name darf nicht leer sein"),
                );
                v.require(
                    iname.chars().count() <= NAME_MAX,
                    "groups",
                    format!(
                        "Gruppe {g}, Punkt {i}: Name darf höchstens {NAME_MAX} Zeichen lang sein"
                    ),
                );
                let description = item.description.trim().to_string();
                v.require(
                    description.chars().count() <= DESCRIPTION_MAX,
                    "groups",
                    format!(
                        "Gruppe {g}, Punkt {i}: Beschreibung darf höchstens {DESCRIPTION_MAX} \
                         Zeichen lang sein"
                    ),
                );
                match to_half(item.max_points) {
                    Some(h) if h > 0 && item.max_points <= POINTS_MAX => {
                        total += h;
                        items.push(CleanItem {
                            name: iname,
                            description,
                            half: h,
                        });
                    }
                    _ => v.add(
                        "groups",
                        format!(
                            "Gruppe {g}, Punkt {i}: Höchstpunktzahl muss größer als 0 und \
                             höchstens {POINTS_MAX} sein, in ganzen oder halben Punkten"
                        ),
                    ),
                }
            }
            groups.push((gname, items));
        }

        // Ist das Schema fehlerhaft, stimmt die Gesamtpunktzahl nicht — dann
        // wäre „ab liegt darüber“ ein Folgefehler, den niemand versteht.
        let schema_ok = v.count() == errors_before;
        v.require(
            !self.grades.is_empty(),
            "grades",
            "mindestens eine Note nötig",
        );
        let mut grades: Vec<(String, i64)> = Vec::new();
        for (n, grade) in self.grades.into_iter().enumerate() {
            let gname = grade.name.trim().to_string();
            let pos = n + 1;
            v.require(
                !gname.is_empty(),
                "grades",
                format!("Note {pos}: Name darf nicht leer sein"),
            );
            match to_half(grade.from_points) {
                Some(h) if h >= 0 => {
                    if schema_ok && h > total {
                        v.add(
                            "grades",
                            format!(
                                "Note „{gname}“: ab {} liegt über der Gesamtpunktzahl {}",
                                from_half(h),
                                from_half(total)
                            ),
                        );
                    }
                    grades.push((gname, h));
                }
                _ => v.add(
                    "grades",
                    format!(
                        "Note „{gname}“: „ab“ muss 0 oder mehr sein, in ganzen oder halben Punkten"
                    ),
                ),
            }
        }
        grades.sort_by_key(|g| g.1);
        for pair in grades.windows(2) {
            v.require(
                pair[0].1 != pair[1].1,
                "grades",
                format!(
                    "„{}“ und „{}“ beginnen beide bei {} Punkten",
                    pair[0].0,
                    pair[1].0,
                    from_half(pair[0].1)
                ),
            );
        }
        v.require(
            grades.first().is_none_or(|g| g.1 == 0),
            "grades",
            "die niedrigste Note muss bei 0 Punkten beginnen",
        );
        v.finish()?;

        Ok(Clean {
            name,
            class_ids,
            with_notes: self.with_notes,
            groups,
            grades,
        })
    }
}

// --- Vorschau --------------------------------------------------------------

/// Die berechneten Werte eines noch nicht gespeicherten Entwurfs.
///
/// Anders als [`create`] lehnt die Vorschau nichts ab: Unlesbares oder
/// Ungültiges ergibt `None` an genau dieser Stelle, alles Übrige wird
/// trotzdem gerechnet. Geschrieben wird nichts.
#[derive(Debug, Clone, PartialEq, Serialize, TS)]
pub struct Preview {
    /// Je Gruppe (in der Reihenfolge der Eingabe): Summe der gültigen Punkte.
    pub group_max_points: Vec<f64>,
    /// Summe aller gültigen Punkte.
    pub max_points: f64,
    /// Je Note (in der Reihenfolge der Eingabe): das „bis“, oder `None`, wenn
    /// das „ab“ ungültig ist oder eine andere Note bei demselben „ab“ beginnt.
    pub grade_to_points: Vec<Option<f64>>,
}

/// Rechnet den Entwurf durch, ohne ihn zu prüfen oder zu speichern.
pub fn preview(input: &ExamInput) -> Preview {
    let mut total: i64 = 0;
    let group_max_points = input
        .groups
        .iter()
        .map(|g| {
            let sum: i64 = g
                .items
                .iter()
                .filter(|i| i.max_points <= POINTS_MAX)
                .filter_map(|i| to_half(i.max_points).filter(|h| *h > 0))
                .sum();
            total += sum;
            from_half(sum)
        })
        .collect();

    let froms: Vec<Option<i64>> = input
        .grades
        .iter()
        .map(|g| to_half(g.from_points).filter(|h| *h >= 0))
        .collect();
    let grade_to_points = froms
        .iter()
        .map(|from| {
            let from = (*from)?;
            let same = froms.iter().flatten().filter(|o| **o == from).count();
            if same > 1 {
                return None;
            }
            let next = froms.iter().flatten().filter(|o| **o > from).min();
            Some(from_half(next.map_or(total, |n| n - 1)))
        })
        .collect();

    Preview {
        group_max_points,
        max_points: from_half(total),
        grade_to_points,
    }
}

// --- Lesen -----------------------------------------------------------------

/// Neueste Prüfung zuerst.
pub fn list(conn: &Connection) -> Result<Vec<Exam>> {
    let mut stmt = conn.prepare("SELECT id FROM exams ORDER BY created_at DESC, id DESC")?;
    let ids: Vec<String> = stmt
        .query_map([], |r| r.get(0))?
        .collect::<std::result::Result<_, _>>()?;
    ids.iter().map(|id| get(conn, ids::parse(id)?)).collect()
}

pub fn get(conn: &Connection, id: Uuid) -> Result<Exam> {
    let (name, created_at, with_notes): (String, String, bool) = conn
        .query_row(
            "SELECT name, created_at, with_notes FROM exams WHERE id = ?1",
            [id.to_string()],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
        )
        .optional()?
        .ok_or(Error::NotFound)?;

    let class_ids = {
        let mut stmt = conn.prepare(
            "SELECT c.id FROM exam_classes e JOIN classes c ON c.id = e.class_id \
             WHERE e.exam_id = ?1 ORDER BY c.year DESC, c.name COLLATE NOCASE",
        )?;
        let rows: Vec<String> = stmt
            .query_map([id.to_string()], |r| r.get(0))?
            .collect::<std::result::Result<_, _>>()?;
        rows.iter()
            .map(|s| ids::parse(s))
            .collect::<Result<Vec<_>>>()?
    };

    let mut groups = Vec::new();
    let mut total: i64 = 0;
    {
        let mut gstmt =
            conn.prepare("SELECT id, name FROM exam_groups WHERE exam_id = ?1 ORDER BY position")?;
        let group_rows: Vec<(i64, String)> = gstmt
            .query_map([id.to_string()], |r| Ok((r.get(0)?, r.get(1)?)))?
            .collect::<std::result::Result<_, _>>()?;
        let mut istmt = conn.prepare(
            "SELECT name, description, max_half FROM exam_items WHERE group_id = ?1 \
             ORDER BY position",
        )?;
        for (gid, gname) in group_rows {
            let items: Vec<(String, String, i64)> = istmt
                .query_map([gid], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)))?
                .collect::<std::result::Result<_, _>>()?;
            let sum: i64 = items.iter().map(|i| i.2).sum();
            total += sum;
            groups.push(Group {
                name: gname,
                items: items
                    .into_iter()
                    .map(|(name, description, h)| Item {
                        name,
                        description,
                        max_points: from_half(h),
                    })
                    .collect(),
                max_points: from_half(sum),
            });
        }
    }

    let grade_rows: Vec<(String, i64)> = {
        let mut stmt = conn.prepare(
            "SELECT name, from_half FROM exam_grades WHERE exam_id = ?1 ORDER BY from_half",
        )?;
        stmt.query_map([id.to_string()], |r| Ok((r.get(0)?, r.get(1)?)))?
            .collect::<std::result::Result<_, _>>()?
    };
    let grades = grade_rows
        .iter()
        .enumerate()
        .map(|(i, (name, from))| Grade {
            name: name.clone(),
            from_points: from_half(*from),
            // Eine halbe Punktstufe unter dem „ab“ der nächsten Note.
            to_points: from_half(grade_rows.get(i + 1).map_or(total, |next| next.1 - 1)),
        })
        .collect();

    Ok(Exam {
        id,
        name,
        class_ids,
        groups,
        grades,
        max_points: from_half(total),
        with_notes,
        created_at: timestamp::from_text(&created_at)?,
    })
}

// --- Schreiben -------------------------------------------------------------

pub fn create(conn: &Connection, input: ExamInput) -> Result<Exam> {
    let clean = input.clean()?;
    check_classes(conn, &clean)?;
    let id = Uuid::now_v7();
    let tx = conn.unchecked_transaction()?;
    tx.execute(
        "INSERT INTO exams (id, name, with_notes, created_at) VALUES (?1, ?2, ?3, ?4)",
        rusqlite::params![
            id.to_string(),
            clean.name,
            clean.with_notes,
            timestamp::to_text(timestamp::now())
        ],
    )?;
    write_children(&tx, id, &clean)?;
    tx.commit()?;
    tracing::debug!(%id, "Prüfung angelegt");
    get(conn, id)
}

/// Ersetzt Name, Klassen, Schema und Matrix der Prüfung.
pub fn update(conn: &Connection, id: Uuid, input: ExamInput) -> Result<Exam> {
    let clean = input.clean()?;
    get(conn, id)?;
    check_classes(conn, &clean)?;
    let tx = conn.unchecked_transaction()?;
    tx.execute(
        "UPDATE exams SET name = ?2, with_notes = ?3 WHERE id = ?1",
        rusqlite::params![id.to_string(), clean.name, clean.with_notes],
    )?;
    // Gruppen löschen die Punkte mit (ON DELETE CASCADE).
    for table in ["exam_classes", "exam_groups", "exam_grades"] {
        tx.execute(
            &format!("DELETE FROM {table} WHERE exam_id = ?1"),
            [id.to_string()],
        )?;
    }
    write_children(&tx, id, &clean)?;
    // Bereits eingetragene Bewertungen bleiben, soweit sie noch passen.
    crate::grading::prune(&tx, id)?;
    tx.commit()?;
    get(conn, id)
}

pub fn delete(conn: &Connection, id: Uuid) -> Result<()> {
    let changed = conn.execute("DELETE FROM exams WHERE id = ?1", [id.to_string()])?;
    if changed == 0 {
        return Err(Error::NotFound);
    }
    tracing::debug!(%id, "Prüfung gelöscht");
    Ok(())
}

pub fn resolve_id(conn: &Connection, needle: &str) -> Result<Uuid> {
    ids::resolve(conn, ids::Table::Exams, needle)
}

fn check_classes(conn: &Connection, clean: &Clean) -> Result<()> {
    let mut v = Validator::new();
    for id in &clean.class_ids {
        v.require(
            class::get(conn, *id).is_ok(),
            "classes",
            "eine gewählte Klasse gibt es nicht mehr",
        );
    }
    v.finish()
}

fn write_children(conn: &Connection, id: Uuid, clean: &Clean) -> Result<()> {
    for class_id in &clean.class_ids {
        conn.execute(
            "INSERT INTO exam_classes (exam_id, class_id) VALUES (?1, ?2)",
            [id.to_string(), class_id.to_string()],
        )?;
    }
    for (gpos, (gname, items)) in clean.groups.iter().enumerate() {
        conn.execute(
            "INSERT INTO exam_groups (exam_id, position, name) VALUES (?1, ?2, ?3)",
            rusqlite::params![id.to_string(), gpos as i64, gname],
        )?;
        let gid = conn.last_insert_rowid();
        for (ipos, item) in items.iter().enumerate() {
            conn.execute(
                "INSERT INTO exam_items (group_id, position, name, description, max_half) \
                 VALUES (?1, ?2, ?3, ?4, ?5)",
                rusqlite::params![gid, ipos as i64, item.name, item.description, item.half],
            )?;
        }
    }
    for (name, half) in &clean.grades {
        conn.execute(
            "INSERT INTO exam_grades (exam_id, name, from_half) VALUES (?1, ?2, ?3)",
            rusqlite::params![id.to_string(), name, half],
        )?;
    }
    Ok(())
}

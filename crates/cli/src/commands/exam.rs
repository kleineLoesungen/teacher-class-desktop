use anyhow::Result;
use teacher_class_core::App;
use teacher_class_core::exam::{self, ExamInput, GradeInput, GroupInput, ItemInput};
use teacher_class_core::{class, export};

use super::confirm::confirm;
use crate::args::ExamCommand;
use crate::output::Out;

pub fn run(cmd: ExamCommand, app: &App, out: &Out) -> Result<()> {
    match cmd {
        ExamCommand::Add {
            name,
            class: class_ids,
            group,
            grade,
        } => {
            let class_ids = class_ids
                .iter()
                .map(|c| class::resolve_id(app.conn(), c))
                .collect::<teacher_class_core::Result<Vec<_>>>()?;
            let created = exam::create(
                app.conn(),
                ExamInput {
                    name,
                    class_ids,
                    groups: group,
                    grades: grade,
                },
            )?;
            out.done(
                format!("Angelegt: {}  {}", Out::short_id(created.id), created.name),
                &created,
            );
            Ok(())
        }

        ExamCommand::List => {
            out.exams(&exam::list(app.conn())?);
            Ok(())
        }

        ExamCommand::Show { id } => {
            let id = exam::resolve_id(app.conn(), &id)?;
            let exam = exam::get(app.conn(), id)?;
            let labels = exam
                .class_ids
                .iter()
                .map(|c| class::get(app.conn(), *c).map(|c| c.label()))
                .collect::<teacher_class_core::Result<Vec<_>>>()?;
            out.exam_detail(&exam, &labels);
            Ok(())
        }

        ExamCommand::Rm { id, yes } => {
            let id = exam::resolve_id(app.conn(), &id)?;
            let existing = exam::get(app.conn(), id)?;

            if !yes && !confirm(&format!("Prüfung „{}\u{201c} löschen?", existing.name))? {
                if !out.json {
                    println!("Abgebrochen.");
                }
                return Ok(());
            }

            exam::delete(app.conn(), id)?;
            out.done(format!("Gelöscht: {}", existing.name), &existing);
            Ok(())
        }

        ExamCommand::Export { id, out: path } => {
            let id = exam::resolve_id(app.conn(), &id)?;
            export::write_xlsx(app.conn(), id, &path)?;
            out.done(
                format!("Geschrieben: {}", path.display()),
                &serde_json::json!({ "path": path }),
            );
            Ok(())
        }
    }
}

fn parse_points(text: &str) -> Result<f64, String> {
    // Komma wie Punkt als Dezimaltrenner, wie man es in Deutschland schreibt.
    text.trim()
        .replace(',', ".")
        .parse::<f64>()
        .map_err(|_| format!("„{}\u{201c} ist keine Zahl", text.trim()))
}

/// `Aufgabe 1: a=4; b=2.5` → Gruppe mit zwei Punkten.
pub fn parse_group(text: &str) -> Result<GroupInput, String> {
    let (name, items) = text
        .split_once(':')
        .ok_or("erwartet „Name: Punkt=Höchstpunkte; …“")?;
    let items = items
        .split(';')
        .filter(|s| !s.trim().is_empty())
        .map(|s| {
            let (name, max) = s
                .rsplit_once('=')
                .ok_or_else(|| format!("„{}\u{201c}: erwartet Punkt=Höchstpunkte", s.trim()))?;
            Ok(ItemInput {
                name: name.trim().to_string(),
                max_points: parse_points(max)?,
            })
        })
        .collect::<Result<Vec<_>, String>>()?;
    Ok(GroupInput {
        name: name.trim().to_string(),
        items,
    })
}

/// `1=17` → Note „1“ ab 17 Punkten.
pub fn parse_grade(text: &str) -> Result<GradeInput, String> {
    let (name, from) = text
        .rsplit_once('=')
        .ok_or("erwartet Note=ab-Punkte, z. B. 1=17")?;
    Ok(GradeInput {
        name: name.trim().to_string(),
        from_points: parse_points(from)?,
    })
}

use std::io::Read;

use anyhow::{Context, Result};
use teacher_class_core::App;
use teacher_class_core::{class, student};

use super::confirm::confirm;
use crate::args::StudentCommand;
use crate::output::Out;

pub fn run(cmd: StudentCommand, app: &App, out: &Out) -> Result<()> {
    match cmd {
        StudentCommand::Add {
            class: class_id,
            text,
        } => {
            let class_id = class::resolve_id(app.conn(), &class_id)?;
            let text = read_text(&text)?;
            let created = student::add_many(app.conn(), class_id, &text)?;
            out.done(format!("{} Schüler angelegt.", created.len()), &created);
            Ok(())
        }

        StudentCommand::List { class: class_id } => {
            let class_id = class::resolve_id(app.conn(), &class_id)?;
            out.students(&student::list(app.conn(), class_id)?);
            Ok(())
        }

        StudentCommand::Rm { id, yes } => {
            let id = student::resolve_id(app.conn(), &id)?;
            let existing = student::get(app.conn(), id)?;

            if !yes && !confirm(&format!("„{}\u{201c} löschen?", existing.label()))? {
                if !out.json {
                    println!("Abgebrochen.");
                }
                return Ok(());
            }

            student::delete(app.conn(), id)?;
            out.done(format!("Gelöscht: {}", existing.label()), &existing);
            Ok(())
        }
    }
}

/// `-` bedeutet: den Text von stdin lesen.
fn read_text(arg: &str) -> Result<String> {
    if arg != "-" {
        return Ok(arg.to_string());
    }
    let mut buf = String::new();
    std::io::stdin()
        .read_to_string(&mut buf)
        .context("Text von stdin lesen")?;
    Ok(buf)
}

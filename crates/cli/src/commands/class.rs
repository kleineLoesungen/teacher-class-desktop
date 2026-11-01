use anyhow::Result;
use teacher_class_core::App;
use teacher_class_core::class::{self, ClassInput};

use super::confirm::confirm;
use crate::args::ClassCommand;
use crate::output::Out;

fn archive(app: &App, out: &Out, id: &str, archived: bool) -> Result<()> {
    let id = class::resolve_id(app.conn(), id)?;
    let updated = class::set_archived(app.conn(), id, archived)?;
    let verb = if archived {
        "Archiviert"
    } else {
        "Zurückgeholt"
    };
    out.done(format!("{verb}: {}", updated.label()), &updated);
    Ok(())
}

pub fn run(cmd: ClassCommand, app: &App, out: &Out) -> Result<()> {
    match cmd {
        ClassCommand::Add { name, year } => {
            let created = class::create(app.conn(), ClassInput::new(name, year))?;
            out.done(
                format!(
                    "Angelegt: {}  {}",
                    Out::short_id(created.id),
                    created.label()
                ),
                &created,
            );
            Ok(())
        }

        ClassCommand::List => {
            out.classes(&class::list(app.conn())?);
            Ok(())
        }

        ClassCommand::Edit { id, name, year } => {
            let id = class::resolve_id(app.conn(), &id)?;
            let current = class::get(app.conn(), id)?;
            // Nicht angegebene Felder behalten ihren Wert.
            let input = ClassInput::new(name.unwrap_or(current.name), year.unwrap_or(current.year));
            let updated = class::update(app.conn(), id, input)?;
            out.done(format!("Geändert: {}", updated.label()), &updated);
            Ok(())
        }

        ClassCommand::Archive { id } => archive(app, out, &id, true),
        ClassCommand::Unarchive { id } => archive(app, out, &id, false),

        ClassCommand::Rm { id, yes } => {
            let id = class::resolve_id(app.conn(), &id)?;
            let existing = class::get(app.conn(), id)?;

            if !yes
                && !confirm(&format!(
                    "Klasse „{}\u{201c} mit {} Schülern löschen?",
                    existing.label(),
                    existing.student_count
                ))?
            {
                if !out.json {
                    println!("Abgebrochen.");
                }
                return Ok(());
            }

            class::delete(app.conn(), id)?;
            out.done(format!("Gelöscht: {}", existing.label()), &existing);
            Ok(())
        }
    }
}

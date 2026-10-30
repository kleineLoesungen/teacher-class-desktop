use std::io::{IsTerminal, Read, Write};

use anyhow::{Context, Result, bail};
use starter_core::App;
use starter_core::note::{self, NoteInput};

use crate::args::NoteCommand;
use crate::output::Out;

pub fn run(cmd: NoteCommand, app: &App, out: &Out) -> Result<()> {
    match cmd {
        NoteCommand::Add(fields) => {
            let body = read_body(&fields.body)?;
            let created = note::create(app.conn(), NoteInput::new(fields.title, body))?;
            out.done(
                format!("Angelegt: {}  {}", Out::short_id(&created), created.title),
                &created,
            );
            Ok(())
        }

        NoteCommand::List { order } => {
            let notes = note::list(app.conn(), order.into())?;
            out.notes(&notes);
            Ok(())
        }

        NoteCommand::Show { id } => {
            let id = note::resolve_id(app.conn(), &id)?;
            out.note(&note::get(app.conn(), id)?);
            Ok(())
        }

        NoteCommand::Edit { id, title, body } => {
            let id = note::resolve_id(app.conn(), &id)?;
            let current = note::get(app.conn(), id)?;

            // Nicht angegebene Felder behalten ihren Wert. Wer `--body ""`
            // schreibt, leert den Text absichtlich — das ist etwas anderes,
            // als die Option wegzulassen.
            let body = match body {
                Some(b) => read_body(&b)?,
                None => current.body.clone(),
            };
            let input = NoteInput::new(title.unwrap_or(current.title), body);

            let updated = note::update(app.conn(), id, input)?;
            out.done(format!("Geändert: {}", Out::short_id(&updated)), &updated);
            Ok(())
        }

        NoteCommand::Rm { id, yes } => {
            let id = note::resolve_id(app.conn(), &id)?;
            let existing = note::get(app.conn(), id)?;

            if !yes && !confirm(&format!("„{}\u{201c} löschen?", existing.title))? {
                // Abbruch auf Wunsch des Benutzers ist kein Fehler.
                if !out.json {
                    println!("Abgebrochen.");
                }
                return Ok(());
            }

            note::delete(app.conn(), id)?;
            out.done(format!("Gelöscht: {}", Out::short_id(&existing)), &existing);
            Ok(())
        }
    }
}

/// `-` bedeutet: den Text von stdin lesen.
///
/// Damit geht `cat notiz.md | starter note add "Titel" --body -` und die
/// Anwendung bleibt in Pipelines brauchbar.
fn read_body(arg: &str) -> Result<String> {
    if arg != "-" {
        return Ok(arg.to_string());
    }
    let mut buf = String::new();
    std::io::stdin()
        .read_to_string(&mut buf)
        .context("Text von stdin lesen")?;
    Ok(buf)
}

/// Rückfrage vor einer nicht umkehrbaren Aktion.
///
/// Ohne Terminal — im Skript, in der CI — gibt es keine Rückfrage, sondern
/// einen Fehler mit dem Hinweis auf `--yes`. Stillschweigend zu löschen, weil
/// niemand antworten kann, wäre die falsche Voreinstellung.
fn confirm(question: &str) -> Result<bool> {
    if !std::io::stdin().is_terminal() {
        bail!("{question} Keine Rückfrage möglich (kein Terminal) — mit --yes bestätigen.");
    }
    print!("{question} [j/N] ");
    std::io::stdout().flush().ok();

    let mut answer = String::new();
    std::io::stdin()
        .read_line(&mut answer)
        .context("Antwort lesen")?;
    Ok(matches!(answer.trim().to_lowercase().as_str(), "j" | "ja"))
}

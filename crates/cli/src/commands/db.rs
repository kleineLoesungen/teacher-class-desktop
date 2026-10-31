use anyhow::{Context, Result};
use teacher_class_core::{App, Paths};

use crate::args::DbCommand;
use crate::output::Out;

/// Läuft **vor** `App::open`: Gerade wenn am eingestellten Ort keine Datenbank
/// liegt, soll sich der Ort hiermit ändern lassen.
pub fn run(cmd: DbCommand, base: &Paths, out: &Out) -> Result<()> {
    let app = match cmd {
        DbCommand::Use { file } => App::use_database(base, &absolute(&file)?)?,
        DbCommand::Create { file } => App::create_database(base, &absolute(&file)?)?,
        DbCommand::Reset => App::use_default_database(base)?,
    };
    let file = app.paths().db_file();
    out.done(
        format!("Datenbank: {}", file.display()),
        &serde_json::json!({ "db_file": file }),
    );
    Ok(())
}

/// Die Einstellungsdatei verlangt einen vollständigen Pfad; auf der
/// Kommandozeile ist ein relativer der Normalfall.
fn absolute(file: &std::path::Path) -> Result<std::path::PathBuf> {
    std::path::absolute(file).with_context(|| format!("{} auflösen", file.display()))
}

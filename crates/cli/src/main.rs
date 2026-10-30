//! Die Kommandozeilenschale.
//!
//! Aufgabe dieser Datei: Argumente parsen, eine [`App`] aufbauen, den passenden
//! Befehl rufen, Fehler in einen Rückgabewert übersetzen. Mehr nicht.
//!
//! Warum `ExitCode` statt `Result` in `main`: Bei `fn main() -> Result<…>`
//! druckt Rust die Debug-Fassung des Fehlers und liefert immer 1. Wir wollen
//! eine lesbare Meldung und eine Nummer, auf die ein Skript reagieren kann —
//! siehe `exit.rs`.

mod args;
mod commands;
mod exit;
mod output;

use std::io::Write;
use std::process::ExitCode;

use anyhow::{Context, Result};
use clap::{CommandFactory, Parser};
use starter_core::{App, Paths};

use crate::args::{Cli, Command};
use crate::output::Out;

fn main() -> ExitCode {
    let cli = Cli::parse();
    init_tracing(cli.verbose);

    let json = cli.json;
    match run(cli) {
        Ok(()) => ExitCode::SUCCESS,
        Err(err) => exit::report(&err, json),
    }
}

fn run(cli: Cli) -> Result<()> {
    let out = Out { json: cli.json };

    // Befehle, die keine Datenbank brauchen, kommen vor dem Öffnen — sonst
    // scheitert `starter completions zsh` auf einem Rechner ohne Schreibrecht
    // im Datenverzeichnis.
    match cli.command {
        Command::Completions { shell } => return completions(shell),
        Command::Man { out: dir } => return man(&dir),
        _ => {}
    }

    let paths = resolve_paths(&cli)?;

    match cli.command {
        Command::Paths => {
            out.paths(&paths);
            Ok(())
        }
        Command::Note(cmd) => {
            let app = App::open(paths)?;
            commands::note::run(cmd, &app, &out)
        }
        Command::Config(cmd) => {
            let mut app = App::open(paths)?;
            commands::config::run(cmd, &mut app, &out)
        }
        Command::Completions { .. } | Command::Man { .. } => unreachable!("oben behandelt"),
    }
}

/// `--data-dir` und `--config-dir` schlagen die Voreinstellung der Plattform.
///
/// Sind beide gesetzt, wird die Plattform gar nicht erst befragt — so läuft
/// die Anwendung auch dort, wo es kein Heimatverzeichnis gibt.
fn resolve_paths(cli: &Cli) -> Result<Paths> {
    Ok(match (&cli.data_dir, &cli.config_dir) {
        (Some(data), Some(config)) => Paths::from_parts(data.clone(), config.clone()),
        (data, config) => {
            let base = Paths::resolve()?;
            Paths::from_parts(
                data.clone()
                    .unwrap_or_else(|| base.data_dir().to_path_buf()),
                config
                    .clone()
                    .unwrap_or_else(|| base.config_dir().to_path_buf()),
            )
        }
    })
}

/// Protokoll auf stderr, niemals auf stdout.
///
/// stdout gehört der Nutzausgabe. Wer dort protokolliert, macht `--json`
/// unbrauchbar, weil `jq` über die Protokollzeile stolpert.
fn init_tracing(verbose: u8) {
    let level = match verbose {
        0 => "warn",
        1 => "info",
        2 => "debug",
        _ => "trace",
    };
    let filter = tracing_subscriber::EnvFilter::try_from_env("STARTER_LOG")
        .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new(format!("starter={level}")));

    tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_writer(std::io::stderr)
        .without_time()
        .with_target(false)
        .init();
}

fn completions(shell: clap_complete::Shell) -> Result<()> {
    let mut cmd = Cli::command();
    let name = cmd.get_name().to_string();
    clap_complete::generate(shell, &mut cmd, name, &mut std::io::stdout());
    Ok(())
}

/// Schreibt je eine Handbuchseite für den Hauptbefehl und jeden Unterbefehl.
fn man(dir: &std::path::Path) -> Result<()> {
    std::fs::create_dir_all(dir).with_context(|| format!("{} anlegen", dir.display()))?;
    let cmd = Cli::command();
    write_man_recursive(&cmd, cmd.get_name().to_string(), dir)?;
    println!("Handbuchseiten in {}", dir.display());
    Ok(())
}

fn write_man_recursive(cmd: &clap::Command, name: String, dir: &std::path::Path) -> Result<()> {
    let path = dir.join(format!("{name}.1"));
    let mut buf = Vec::new();
    clap_mangen::Man::new(cmd.clone().name(name.clone()))
        .render(&mut buf)
        .with_context(|| format!("Handbuchseite {name} erzeugen"))?;
    let mut file =
        std::fs::File::create(&path).with_context(|| format!("{} schreiben", path.display()))?;
    file.write_all(&buf)
        .with_context(|| format!("{} schreiben", path.display()))?;

    for sub in cmd.get_subcommands() {
        write_man_recursive(sub, format!("{name}-{}", sub.get_name()), dir)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// clap findet zur Übersetzungszeit nichts von dem, was erst zur Laufzeit
    /// schiefgehen kann — doppelte Kurzoptionen etwa. Dieser Test tut es.
    #[test]
    fn befehlsstruktur_ist_gueltig() {
        Cli::command().debug_assert();
    }
}

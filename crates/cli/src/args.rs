//! Die Befehlsstruktur.
//!
//! Hier steht **nur**, wie Argumente aussehen. Was passiert, steht in
//! `commands/`. Die Trennung hält diese Datei lesbar und erlaubt es, die
//! Hilfetexte zu überfliegen, ohne Logik zu lesen.
//!
//! Hilfetexte sind deutsch, Bezeichner und Optionsnamen englisch — wie überall
//! im Kit.

use std::path::PathBuf;

use clap::{Parser, Subcommand};

#[derive(Debug, Parser)]
#[command(
    name = "teacher-class",
    version,
    about = "Klassen, Schüler und Prüfungen für Lehrkräfte",
    long_about = "Verwaltet Klassen und Schüler. Dieselbe Fachlogik bedient die \
                  Oberfläche `teacher-class-desktop`.",
    propagate_version = true
)]
pub struct Cli {
    /// Ausgabe als JSON statt für Menschen.
    ///
    /// Gilt auch für Fehler: dann steht auf stderr ein JSON-Objekt mit
    /// `error.code`, `error.message` und gegebenenfalls `error.fields`.
    #[arg(long, global = true)]
    pub json: bool,

    /// Mehr Protokoll auf stderr. Zweimal für noch mehr.
    #[arg(short, long, global = true, action = clap::ArgAction::Count)]
    pub verbose: u8,

    /// Datenverzeichnis überschreiben (sonst der Ort der Plattform).
    #[arg(
        long,
        global = true,
        value_name = "VERZEICHNIS",
        env = "TEACHER_CLASS_DATA_DIR"
    )]
    pub data_dir: Option<PathBuf>,

    /// Konfigurationsverzeichnis überschreiben.
    #[arg(
        long,
        global = true,
        value_name = "VERZEICHNIS",
        env = "TEACHER_CLASS_CONFIG_DIR"
    )]
    pub config_dir: Option<PathBuf>,

    #[command(subcommand)]
    pub command: Command,
}

#[derive(Debug, Subcommand)]
pub enum Command {
    /// Klassen verwalten.
    #[command(subcommand)]
    Class(ClassCommand),

    /// Schüler einer Klasse verwalten.
    #[command(subcommand)]
    Student(StudentCommand),

    /// Einstellungen anzeigen und ändern.
    #[command(subcommand)]
    Config(ConfigCommand),

    /// Zeigt, wo die Anwendung ihre Dateien ablegt.
    Paths,

    /// Vervollständigung für die Shell ausgeben.
    ///
    /// Beispiel: `teacher-class completions zsh > ~/.zfunc/_teacher-class`
    Completions {
        #[arg(value_enum)]
        shell: Shell,
    },

    /// Handbuchseiten nach VERZEICHNIS schreiben.
    Man {
        #[arg(long, value_name = "VERZEICHNIS", default_value = "dist/man")]
        out: PathBuf,
    },
}

#[derive(Debug, Subcommand)]
pub enum ClassCommand {
    /// Neue Klasse anlegen, z. B. `class add 7a 2025`.
    Add {
        /// Name: nur Buchstaben und Zahlen.
        name: String,

        /// Vierstelliges Jahr.
        year: i32,
    },

    /// Alle Klassen auflisten.
    #[command(alias = "ls")]
    List,

    /// Name oder Jahr einer Klasse ändern.
    Edit {
        /// Kennung oder ein eindeutiges Ende davon.
        id: String,

        #[arg(long)]
        name: Option<String>,

        #[arg(long)]
        year: Option<i32>,
    },

    /// Eine Klasse samt ihren Schülern löschen.
    #[command(alias = "delete")]
    Rm {
        /// Kennung oder ein eindeutiges Ende davon.
        id: String,

        /// Nicht nachfragen.
        #[arg(short = 'y', long)]
        yes: bool,
    },
}

#[derive(Debug, Subcommand)]
pub enum StudentCommand {
    /// Schüler anlegen: eine Zeile `Nachname, Vorname` je Schüler.
    Add {
        /// Kennung der Klasse oder ein eindeutiges Ende davon.
        class: String,

        /// Die Namen, getrennt durch Zeilenumbrüche. `-` liest von stdin.
        text: String,
    },

    /// Die Schüler einer Klasse auflisten.
    #[command(alias = "ls")]
    List {
        /// Kennung der Klasse oder ein eindeutiges Ende davon.
        class: String,
    },

    /// Einen Schüler löschen.
    #[command(alias = "delete")]
    Rm {
        /// Kennung oder ein eindeutiges Ende davon.
        id: String,

        /// Nicht nachfragen.
        #[arg(short = 'y', long)]
        yes: bool,
    },
}

#[derive(Debug, Subcommand)]
pub enum ConfigCommand {
    /// Aktuelle Einstellungen anzeigen.
    Show,

    /// Eine Einstellung setzen.
    ///
    /// Schlüssel: `app_name`, `theme` (system|light|dark), `accent` (#rrggbb).
    Set { key: String, value: String },

    /// Einstellungen auf die Voreinstellung zurücksetzen.
    Reset,
}

pub use clap_complete::Shell;

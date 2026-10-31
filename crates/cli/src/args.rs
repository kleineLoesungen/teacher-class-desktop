//! Die Befehlsstruktur.
//!
//! Hier steht **nur**, wie Argumente aussehen. Was passiert, steht in
//! `commands/`. Die Trennung hält diese Datei lesbar und erlaubt es, die
//! Hilfetexte zu überfliegen, ohne Logik zu lesen.
//!
//! Hilfetexte sind deutsch, Bezeichner und Optionsnamen englisch — wie überall
//! in der App.

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

    /// Prüfungen verwalten und als Excel exportieren.
    #[command(subcommand)]
    Exam(ExamCommand),

    /// Einstellungen anzeigen und ändern.
    #[command(subcommand)]
    Config(ConfigCommand),

    /// Speicherort der Datenbank wählen.
    #[command(subcommand)]
    Db(DbCommand),

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
pub enum ExamCommand {
    /// Neue Prüfung anlegen.
    ///
    /// Beispiel:
    ///   exam add "Klassenarbeit 1" --class a1b2c3d4 \
    ///     --group "Aufgabe 1: a=4; b=2.5" --group "Aufgabe 2: a=13.5" \
    ///     --grade 6=0 --grade 3=10 --grade 1=17
    ///
    /// Punkte sind ganz oder halb. Bei den Noten wird nur „ab“ angegeben;
    /// „bis“ berechnet sich. Die niedrigste Note beginnt bei 0.
    ///
    /// Zu jedem Punkt kann eine Beschreibung folgen, abgetrennt mit `|`:
    ///   --group "Aufgabe 1: a=4|Rechenweg angeben; b=2.5"
    Add {
        /// Name der Prüfung.
        name: String,

        /// Kennung einer Klasse oder ein eindeutiges Ende davon. Mehrfach
        /// möglich.
        #[arg(long, required = true, value_name = "KLASSE")]
        class: Vec<String>,

        /// Eine Gruppe: `Name: Punkt=Höchstpunkte|Beschreibung; Punkt=Höchstpunkte`.
        #[arg(
            long,
            required = true,
            value_name = "GRUPPE",
            value_parser = crate::commands::exam::parse_group
        )]
        group: Vec<teacher_class_core::exam::GroupInput>,

        /// Eine Note: `Note=ab-Punkte`.
        #[arg(
            long,
            required = true,
            value_name = "NOTE=AB",
            value_parser = crate::commands::exam::parse_grade
        )]
        grade: Vec<teacher_class_core::exam::GradeInput>,

        /// Zu jedem Punkt eine Begründung pflegen. Der Excel-Export hat dann
        /// eine Bewertungskachel je Schüler statt einer Matrix.
        #[arg(long)]
        notes: bool,
    },

    /// Punkte und Begründung eines Schülers zu einem Prüfkriterium eintragen.
    ///
    /// Gruppe und Punkt zählen ab 1, wie in `exam show`. Ohne `--points` werden
    /// die Punkte gelöscht; ohne `--note` die Begründung.
    ///
    /// Beispiel:  exam score 3f9a2b1c 7d41e0aa 1 2 --points 2.5 --note "Rechenweg fehlt"
    Score {
        /// Kennung der Prüfung oder ein eindeutiges Ende davon.
        id: String,

        /// Kennung des Schülers oder ein eindeutiges Ende davon.
        student: String,

        /// Nummer der Gruppe (ab 1).
        group: u32,

        /// Nummer des Punktes in der Gruppe (ab 1).
        item: u32,

        /// Erreichte Punkte, ganz oder halb.
        #[arg(long, value_parser = crate::commands::exam::parse_points)]
        points: Option<f64>,

        /// Begründung. Nur für Prüfungen mit `--notes`.
        #[arg(long, default_value = "")]
        note: String,
    },

    /// Die Bewertung der Schüler einer Klasse zeigen.
    Results {
        /// Kennung der Prüfung oder ein eindeutiges Ende davon.
        id: String,

        /// Kennung der Klasse oder ein eindeutiges Ende davon.
        #[arg(long, value_name = "KLASSE")]
        class: String,
    },

    /// Alle Prüfungen auflisten.
    #[command(alias = "ls")]
    List,

    /// Schema und Bewertungsmatrix einer Prüfung anzeigen.
    Show {
        /// Kennung oder ein eindeutiges Ende davon.
        id: String,
    },

    /// Eine Prüfung löschen.
    #[command(alias = "delete")]
    Rm {
        /// Kennung oder ein eindeutiges Ende davon.
        id: String,

        /// Nicht nachfragen.
        #[arg(short = 'y', long)]
        yes: bool,
    },

    /// Eine Prüfung als Excel-Datei (.xlsx) exportieren.
    Export {
        /// Kennung oder ein eindeutiges Ende davon.
        id: String,

        /// Zieldatei, muss auf .xlsx enden.
        #[arg(long, value_name = "DATEI")]
        out: PathBuf,
    },
}

#[derive(Debug, Subcommand)]
pub enum DbCommand {
    /// Eine vorhandene Datenbankdatei als Speicherort nehmen.
    ///
    /// Gilt auch für die Oberfläche, denn der Ort steht in der
    /// Einstellungsdatei. Beispiel: `db use /media/stick/klassen.sqlite3`
    Use {
        /// Die Datenbankdatei.
        file: PathBuf,
    },

    /// Eine neue, leere Datenbank anlegen und als Speicherort nehmen.
    ///
    /// Eine vorhandene Datei wird nie überschrieben.
    Create {
        /// Wo die Datei entstehen soll.
        file: PathBuf,
    },

    /// Zurück zum Standardort der Plattform.
    Reset,
}

#[derive(Debug, Subcommand)]
pub enum ConfigCommand {
    /// Aktuelle Einstellungen anzeigen.
    Show,

    /// Eine Einstellung setzen.
    ///
    /// Schlüssel: `app_name`, `theme` (system|light|dark), `accent` (#rrggbb).
    /// Den Speicherort der Datenbank ändert `db`.
    Set { key: String, value: String },

    /// Einstellungen auf die Voreinstellung zurücksetzen.
    Reset,
}

pub use clap_complete::Shell;

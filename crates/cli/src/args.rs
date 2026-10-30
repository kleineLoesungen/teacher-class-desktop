//! Die Befehlsstruktur.
//!
//! Hier steht **nur**, wie Argumente aussehen. Was passiert, steht in
//! `commands/`. Die Trennung hält diese Datei lesbar und erlaubt es, die
//! Hilfetexte zu überfliegen, ohne Logik zu lesen.
//!
//! Hilfetexte sind deutsch, Bezeichner und Optionsnamen englisch — wie überall
//! im Kit.

use std::path::PathBuf;

use clap::{Args as ClapArgs, Parser, Subcommand, ValueEnum};

#[derive(Debug, Parser)]
#[command(
    name = "starter",
    version,
    about = "Beispielanwendung des Starter-Kits",
    long_about = "Verwaltet Notizen. Dieselbe Fachlogik bedient die Oberfläche \
                  `starter-desktop`.",
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
        env = "STARTER_DATA_DIR"
    )]
    pub data_dir: Option<PathBuf>,

    /// Konfigurationsverzeichnis überschreiben.
    #[arg(
        long,
        global = true,
        value_name = "VERZEICHNIS",
        env = "STARTER_CONFIG_DIR"
    )]
    pub config_dir: Option<PathBuf>,

    #[command(subcommand)]
    pub command: Command,
}

#[derive(Debug, Subcommand)]
pub enum Command {
    /// Notizen verwalten.
    #[command(subcommand)]
    Note(NoteCommand),

    /// Einstellungen anzeigen und ändern.
    #[command(subcommand)]
    Config(ConfigCommand),

    /// Zeigt, wo die Anwendung ihre Dateien ablegt.
    Paths,

    /// Vervollständigung für die Shell ausgeben.
    ///
    /// Beispiel: `starter completions zsh > ~/.zfunc/_starter`
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
pub enum NoteCommand {
    /// Neue Notiz anlegen.
    Add(NoteFields),

    /// Alle Notizen auflisten.
    #[command(alias = "ls")]
    List {
        /// Sortierung.
        #[arg(long, value_enum, default_value_t = Order::Recent)]
        order: Order,
    },

    /// Eine Notiz vollständig anzeigen.
    Show {
        /// Kennung oder ein eindeutiger Anfang davon.
        id: String,
    },

    /// Eine Notiz ändern. Nicht angegebene Felder bleiben, wie sie sind.
    Edit {
        /// Kennung oder ein eindeutiger Anfang davon.
        id: String,

        #[arg(long)]
        title: Option<String>,

        #[arg(long)]
        body: Option<String>,
    },

    /// Eine Notiz löschen.
    #[command(alias = "delete")]
    Rm {
        /// Kennung oder ein eindeutiger Anfang davon.
        id: String,

        /// Nicht nachfragen.
        #[arg(short = 'y', long)]
        yes: bool,
    },
}

#[derive(Debug, ClapArgs)]
pub struct NoteFields {
    /// Titel der Notiz.
    pub title: String,

    /// Text der Notiz. `-` liest von stdin.
    #[arg(long, default_value = "")]
    pub body: String,
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

#[derive(Debug, Clone, Copy, ValueEnum)]
pub enum Order {
    /// Zuletzt geändert zuerst.
    Recent,
    /// Alphabetisch nach Titel.
    Title,
}

impl From<Order> for starter_core::note::Order {
    fn from(o: Order) -> Self {
        match o {
            Order::Recent => starter_core::note::Order::Recent,
            Order::Title => starter_core::note::Order::Title,
        }
    }
}

pub use clap_complete::Shell;

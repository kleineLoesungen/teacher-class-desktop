//! Ausgabe — für Menschen oder als JSON.
//!
//! **Jeder Befehl kann beides.** Wer einen neuen Unterbefehl schreibt, gibt
//! ihm beide Fassungen; ein `--json`, das bei manchen Befehlen nichts tut, ist
//! schlimmer als keins, weil ein Skript es nicht vorhersehen kann.
//!
//! Regeln für die JSON-Fassung:
//!
//! * Listen sind ein Array, kein Objekt mit Zähler — `jq '.[] | .title'`
//!   soll ohne Umweg gehen.
//! * Zeitstempel in UTC als RFC-3339-Text (macht `starter_core` schon).
//! * Für Menschen wird in Ortszeit umgerechnet, für Maschinen nie.

use starter_core::note::{self, Note};
use starter_core::{Config, Paths, timestamp};
use time::macros::format_description;

pub struct Out {
    pub json: bool,
}

impl Out {
    /// Kurzform der Kennung, wie `starter note show <KURZ>` sie wieder
    /// entgegennimmt. Die Begründung für „hinten statt vorn" steht in
    /// [`starter_core::note::short_id`].
    pub fn short_id(note: &Note) -> String {
        note::short_id(note.id)
    }

    fn local(note_time: time::OffsetDateTime) -> String {
        let fmt = format_description!("[day].[month].[year] [hour]:[minute]");
        timestamp::to_local(note_time)
            .format(fmt)
            .unwrap_or_else(|_| timestamp::to_text(note_time))
    }

    pub fn notes(&self, notes: &[Note]) {
        if self.json {
            self.print(notes);
            return;
        }
        if notes.is_empty() {
            println!("Keine Notizen. Anlegen mit:  starter note add \"Titel\"");
            return;
        }
        // Die Spaltenbreite richtet sich nach dem längsten Titel, gedeckelt,
        // damit ein Ausreißer die Tabelle nicht sprengt.
        let width = notes
            .iter()
            .map(|n| n.title.chars().count())
            .max()
            .unwrap_or(5)
            .clamp(5, 50);

        println!("{:<8}  {:<width$}  GEÄNDERT", "ID", "TITEL");
        for note in notes {
            println!(
                "{:<8}  {:<width$}  {}",
                Self::short_id(note),
                truncate(&note.title, width),
                Self::local(note.updated_at)
            );
        }
    }

    pub fn note(&self, note: &Note) {
        if self.json {
            self.print(note);
            return;
        }
        println!("{}", note.title);
        println!("{}", "─".repeat(note.title.chars().count().max(3)));
        if !note.body.is_empty() {
            println!("{}", note.body);
            println!();
        }
        println!("Kennung:   {}", note.id);
        println!("Angelegt:  {}", Self::local(note.created_at));
        println!("Geändert:  {}", Self::local(note.updated_at));
    }

    pub fn config(&self, config: &Config) {
        if self.json {
            self.print(config);
            return;
        }
        println!("app_name  {}", config.app_name);
        println!("theme     {:?}  ({})", config.theme, config.theme.label());
        println!("accent    {}", config.accent);
    }

    pub fn paths(&self, paths: &Paths) {
        if self.json {
            self.print(&serde_json::json!({
                "data_dir": paths.data_dir(),
                "config_dir": paths.config_dir(),
                "db_file": paths.db_file(),
                "config_file": paths.config_file(),
                "log_file": paths.log_file(),
            }));
            return;
        }
        println!("Daten          {}", paths.data_dir().display());
        println!("Konfiguration  {}", paths.config_dir().display());
        println!("Datenbank      {}", paths.db_file().display());
        println!("Protokoll      {}", paths.log_file().display());
    }

    /// Eine Rückmeldung nach einer Änderung.
    ///
    /// Im JSON-Betrieb wird das betroffene Objekt ausgegeben, nicht der Satz —
    /// ein Skript will den Datensatz, keinen deutschen Text.
    pub fn done(&self, message: impl std::fmt::Display, value: &impl serde::Serialize) {
        if self.json {
            self.print(value);
        } else {
            println!("{message}");
        }
    }

    fn print(&self, value: &(impl serde::Serialize + ?Sized)) {
        let text = serde_json::to_string_pretty(value).expect("Ausgabe serialisierbar");
        println!("{text}");
    }
}

fn truncate(s: &str, max: usize) -> String {
    if s.chars().count() <= max {
        return s.to_string();
    }
    let mut out: String = s.chars().take(max.saturating_sub(1)).collect();
    out.push('…');
    out
}

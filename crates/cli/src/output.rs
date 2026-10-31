//! Ausgabe — für Menschen oder als JSON.
//!
//! **Jeder Befehl kann beides.** Wer einen neuen Unterbefehl schreibt, gibt
//! ihm beide Fassungen; ein `--json`, das bei manchen Befehlen nichts tut, ist
//! schlimmer als keins, weil ein Skript es nicht vorhersehen kann.
//!
//! Regeln für die JSON-Fassung:
//!
//! * Listen sind ein Array, kein Objekt mit Zähler — `jq '.[] | .name'`
//!   soll ohne Umweg gehen.
//! * Zeitstempel in UTC als RFC-3339-Text (macht `teacher_class_core` schon).

use teacher_class_core::class::Class;
use teacher_class_core::student::Student;
use teacher_class_core::{Config, Paths, ids};
use uuid::Uuid;

pub struct Out {
    pub json: bool,
}

impl Out {
    /// Kurzform der Kennung, wie die Befehle sie wieder entgegennehmen. Die
    /// Begründung für „hinten statt vorn" steht in [`teacher_class_core::ids`].
    pub fn short_id(id: Uuid) -> String {
        ids::short_id(id)
    }

    pub fn classes(&self, classes: &[Class]) {
        if self.json {
            self.print(classes);
            return;
        }
        if classes.is_empty() {
            println!("Keine Klassen. Anlegen mit:  teacher-class class add 7a 2025");
            return;
        }
        let width = classes
            .iter()
            .map(|c| c.label().chars().count())
            .max()
            .unwrap_or(6)
            .clamp(6, 40);

        println!("{:<8}  {:<width$}  SCHÜLER", "ID", "KLASSE");
        for class in classes {
            println!(
                "{:<8}  {:<width$}  {}",
                Self::short_id(class.id),
                class.label(),
                class.student_count
            );
        }
    }

    pub fn students(&self, students: &[Student]) {
        if self.json {
            self.print(students);
            return;
        }
        if students.is_empty() {
            println!(
                "Keine Schüler. Anlegen mit:  teacher-class student add <KLASSE> \"Nachname, Vorname\""
            );
            return;
        }
        println!("{:<8}  NAME", "ID");
        for student in students {
            println!("{:<8}  {}", Self::short_id(student.id), student.label());
        }
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

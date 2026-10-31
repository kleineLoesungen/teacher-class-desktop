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
use teacher_class_core::exam::Exam;
use teacher_class_core::grading::StudentResult;
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

    pub fn exams(&self, exams: &[Exam]) {
        if self.json {
            self.print(exams);
            return;
        }
        if exams.is_empty() {
            println!(
                "Keine Prüfungen. Anlegen mit:  teacher-class exam add \"Name\" --class <KLASSE> \
                 --group \"Aufgabe 1: a=4\" --grade 6=0 --grade 1=3"
            );
            return;
        }
        let width = exams
            .iter()
            .map(|e| e.name.chars().count())
            .max()
            .unwrap_or(7)
            .clamp(7, 40);
        println!("{:<8}  {:<width$}  KLASSEN  PUNKTE", "ID", "PRÜFUNG");
        for exam in exams {
            println!(
                "{:<8}  {:<width$}  {:>7}  {:>6}",
                Self::short_id(exam.id),
                exam.name,
                exam.class_ids.len(),
                exam.max_points
            );
        }
    }

    /// Schema und Matrix einer Prüfung.
    pub fn exam_detail(&self, exam: &Exam, labels: &[String]) {
        if self.json {
            self.print(exam);
            return;
        }
        println!("{}  ({})", exam.name, Self::short_id(exam.id));
        println!("Klassen: {}", labels.join(", "));
        println!(
            "Begründungen: {}",
            if exam.with_notes { "ja" } else { "nein" }
        );
        println!();
        println!("Prüfungsschema");
        for (gi, group) in exam.groups.iter().enumerate() {
            println!("  {}. {}  (max. {})", gi + 1, group.name, group.max_points);
            for (ii, item) in group.items.iter().enumerate() {
                let label = format!("{}. {}", ii + 1, item.name);
                if item.description.is_empty() {
                    println!("    {:<20} {}", label, item.max_points);
                } else {
                    println!(
                        "    {:<20} {:<6} {}",
                        label, item.max_points, item.description
                    );
                }
            }
        }
        println!("  Gesamt: {}", exam.max_points);
        println!();
        println!("Bewertungsmatrix");
        println!("  {:<12} {:>6}  {:>6}", "NOTE", "AB", "BIS");
        for grade in &exam.grades {
            println!(
                "  {:<12} {:>6}  {:>6}",
                grade.name, grade.from_points, grade.to_points
            );
        }
    }

    /// Die Bewertung der Schüler einer Klasse.
    pub fn results(&self, exam: &Exam, results: &[StudentResult]) {
        if self.json {
            self.print(results);
            return;
        }
        if results.is_empty() {
            println!("Keine Schüler in dieser Klasse.");
            return;
        }
        let width = results
            .iter()
            .map(|r| r.student.label().chars().count())
            .max()
            .unwrap_or(4)
            .clamp(4, 40);
        println!(
            "{:<8}  {:<width$}  {:>6}  NOTE   (max. {})",
            "ID", "NAME", "PUNKTE", exam.max_points
        );
        for r in results {
            println!(
                "{:<8}  {:<width$}  {:>6}  {}",
                Self::short_id(r.student.id),
                r.student.label(),
                r.total,
                r.grade.as_deref().unwrap_or("–")
            );
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

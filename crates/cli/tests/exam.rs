//! Abnahmetests der Kommandozeile für Prüfungen und Excel-Export.
//!
//! Wie `cli.rs`: geprüft wird, was ein Benutzer sieht. Die fachlichen Regeln
//! stehen in `crates/core/tests/exam.rs`.

use std::process::Command;

use tempfile::TempDir;

const FILTERS: &[(&str, &str)] = &[
    (
        r"[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}",
        "[uuid]",
    ),
    (r"\b[0-9a-f]{8}\b", "[id]"),
    (r"\d{4}-\d{2}-\d{2}T[0-9:.]+Z", "[zeit]"),
];

struct Ctx {
    dir: TempDir,
}

struct Output {
    stdout: String,
    stderr: String,
    code: i32,
}

impl Output {
    fn ok(self) -> Self {
        assert_eq!(
            self.code, 0,
            "erwartet 0, bekam {}:\n{}",
            self.code, self.stderr
        );
        self
    }
}

impl Ctx {
    fn new() -> Self {
        Self {
            dir: tempfile::Builder::new()
                .prefix("teacher-class-test-")
                .tempdir()
                .unwrap(),
        }
    }

    fn run(&self, args: &[&str]) -> Output {
        let out = Command::new(env!("CARGO_BIN_EXE_teacher-class"))
            .args(args)
            .arg("--data-dir")
            .arg(self.dir.path().join("data"))
            .arg("--config-dir")
            .arg(self.dir.path().join("config"))
            .stdin(std::process::Stdio::null())
            .output()
            .expect("teacher-class startbar");
        Output {
            stdout: String::from_utf8(out.stdout).unwrap(),
            stderr: String::from_utf8(out.stderr).unwrap(),
            code: out.status.code().expect("kein Signal"),
        }
    }

    /// Kurzkennung aus einer Rückmeldung wie `Angelegt: a1b2c3d4  Name`.
    fn id_of(out: &Output) -> String {
        out.stdout
            .split_whitespace()
            .nth(1)
            .expect("Kurzkennung in der Rückmeldung")
            .to_string()
    }

    fn seed_class(&self) -> String {
        let id = Self::id_of(&self.run(&["class", "add", "7a", "2025"]).ok());
        self.run(&["student", "add", &id, "Muster, Max\nBeispiel, Eva"])
            .ok();
        id
    }

    fn seed_exam(&self, class: &str) -> String {
        Self::id_of(
            &self
                .run(&[
                    "exam",
                    "add",
                    "Klassenarbeit 1",
                    "--class",
                    class,
                    "--group",
                    "Aufgabe 1: a=4; b=2,5",
                    "--group",
                    "Aufgabe 2: a=13.5",
                    "--grade",
                    "6=0",
                    "--grade",
                    "3=10",
                    "--grade",
                    "1=17",
                ])
                .ok(),
        )
    }
}

fn snapshot(name: &str, text: &str) {
    let text = text
        .lines()
        .map(str::trim_end)
        .collect::<Vec<_>>()
        .join("\n");
    insta::with_settings!({filters => FILTERS.to_vec()}, {
        insta::assert_snapshot!(name, text);
    });
}

#[test]
fn pruefung_zeigt_schema_und_berechnete_matrix() {
    let ctx = Ctx::new();
    let class = ctx.seed_class();
    let id = ctx.seed_exam(&class);
    snapshot("pruefung", &ctx.run(&["exam", "show", &id]).ok().stdout);
}

#[test]
fn pruefungsliste_ist_eine_tabelle() {
    let ctx = Ctx::new();
    let class = ctx.seed_class();
    ctx.seed_exam(&class);
    snapshot("pruefungen", &ctx.run(&["exam", "list"]).ok().stdout);
}

#[test]
fn leere_pruefungsliste_sagt_wie_es_weitergeht() {
    let ctx = Ctx::new();
    let out = ctx.run(&["exam", "list"]).ok().stdout;
    assert!(out.contains("exam add"), "{out}");
}

#[test]
fn pruefungsliste_als_json_ist_ein_array() {
    let ctx = Ctx::new();
    let class = ctx.seed_class();
    ctx.seed_exam(&class);
    let out = ctx.run(&["exam", "list", "--json"]).ok().stdout;
    let list: serde_json::Value = serde_json::from_str(&out).unwrap();
    assert_eq!(list[0]["max_points"], 20.0);
    assert_eq!(list[0]["grades"][0]["to_points"], 9.5);
}

#[test]
fn die_niedrigste_note_braucht_null_punkte() {
    let ctx = Ctx::new();
    let class = ctx.seed_class();
    let out = ctx.run(&[
        "exam", "add", "Test", "--class", &class, "--group", "A: a=4", "--grade", "1=2",
    ]);
    assert_eq!(out.code, 4, "{}", out.stderr);
    assert!(out.stderr.contains("0 Punkten"), "{}", out.stderr);
}

#[test]
fn eine_unlesbare_gruppe_ist_ein_aufrufsfehler() {
    let ctx = Ctx::new();
    let class = ctx.seed_class();
    let out = ctx.run(&[
        "exam",
        "add",
        "Test",
        "--class",
        &class,
        "--group",
        "ohne Doppelpunkt",
        "--grade",
        "1=0",
    ]);
    assert_eq!(out.code, 2, "{}", out.stderr);
}

#[test]
fn export_schreibt_eine_xlsx_datei() {
    let ctx = Ctx::new();
    let class = ctx.seed_class();
    let id = ctx.seed_exam(&class);
    let target = ctx.dir.path().join("pruefung.xlsx");
    ctx.run(&["exam", "export", &id, "--out", target.to_str().unwrap()])
        .ok();
    let bytes = std::fs::read(&target).unwrap();
    assert_eq!(&bytes[..2], b"PK");

    let wrong = ctx.dir.path().join("pruefung.csv");
    let out = ctx.run(&["exam", "export", &id, "--out", wrong.to_str().unwrap()]);
    assert_eq!(out.code, 4, "{}", out.stderr);
}

/// Prüfung mit Beschreibung und Begründungen; gibt (Prüfung, Max) zurück.
fn seed_graded(ctx: &Ctx, class: &str) -> (String, String) {
    let exam = Ctx::id_of(
        &ctx.run(&[
            "exam",
            "add",
            "Klassenarbeit 1",
            "--class",
            class,
            "--group",
            "Aufgabe 1: a=4|Rechenweg angeben; b=2,5",
            "--group",
            "Aufgabe 2: a=13.5",
            "--grade",
            "6=0",
            "--grade",
            "3=10",
            "--grade",
            "1=17",
            "--notes",
        ])
        .ok(),
    );
    let list = ctx.run(&["student", "list", class]).ok().stdout;
    let max = list
        .lines()
        .find(|l| l.contains("Muster, Max"))
        .and_then(|l| l.split_whitespace().next())
        .expect("Max in der Liste")
        .to_string();
    (exam, max)
}

#[test]
fn die_beschreibung_steht_in_der_pruefungsansicht() {
    let ctx = Ctx::new();
    let class = ctx.seed_class();
    let (exam, _) = seed_graded(&ctx, &class);
    snapshot(
        "pruefung_mit_beschreibung",
        &ctx.run(&["exam", "show", &exam]).ok().stdout,
    );
}

#[test]
fn bewerten_traegt_ein_und_die_ergebnisse_zeigen_summe_und_note() {
    let ctx = Ctx::new();
    let class = ctx.seed_class();
    let (exam, max) = seed_graded(&ctx, &class);

    ctx.run(&[
        "exam",
        "score",
        &exam,
        &max,
        "1",
        "1",
        "--points",
        "4",
        "--note",
        "Rechenweg ok",
    ])
    .ok();
    ctx.run(&["exam", "score", &exam, &max, "2", "1", "--points", "13,5"])
        .ok();

    snapshot(
        "ergebnisse",
        &ctx.run(&["exam", "results", &exam, "--class", &class])
            .ok()
            .stdout,
    );

    let json = ctx
        .run(&["exam", "results", &exam, "--class", &class, "--json"])
        .ok()
        .stdout;
    let results: serde_json::Value = serde_json::from_str(&json).unwrap();
    let max = results
        .as_array()
        .unwrap()
        .iter()
        .find(|r| r["student"]["last_name"] == "Muster")
        .unwrap();
    assert_eq!(max["total"], 17.5);
    assert_eq!(max["grade"], "1");
    assert_eq!(max["groups"][0]["scores"][0]["note"], "Rechenweg ok");
}

#[test]
fn ungueltige_punkte_sind_ein_eingabefehler() {
    let ctx = Ctx::new();
    let class = ctx.seed_class();
    let (exam, max) = seed_graded(&ctx, &class);
    let out = ctx.run(&["exam", "score", &exam, &max, "1", "1", "--points", "5"]);
    assert_eq!(out.code, 4, "{}", out.stderr);
    // Gruppe 0 gibt es auf der Kommandozeile nicht (sie zählt ab 1).
    let out = ctx.run(&["exam", "score", &exam, &max, "0", "1", "--points", "1"]);
    assert_eq!(out.code, 4, "{}", out.stderr);
}

#[test]
fn begruendungen_ohne_notes_werden_abgelehnt() {
    let ctx = Ctx::new();
    let class = ctx.seed_class();
    let exam = ctx.seed_exam(&class);
    let list = ctx.run(&["student", "list", &class]).ok().stdout;
    let max = list
        .lines()
        .find(|l| l.contains("Muster, Max"))
        .and_then(|l| l.split_whitespace().next())
        .unwrap()
        .to_string();
    let out = ctx.run(&[
        "exam", "score", &exam, &max, "1", "1", "--points", "1", "--note", "x",
    ]);
    assert_eq!(out.code, 4, "{}", out.stderr);
}

#[test]
fn loeschen_verlangt_bestaetigung() {
    let ctx = Ctx::new();
    let class = ctx.seed_class();
    let id = ctx.seed_exam(&class);
    let out = ctx.run(&["exam", "rm", &id]);
    assert_ne!(out.code, 0);
    assert!(out.stderr.contains("--yes"), "{}", out.stderr);
    ctx.run(&["exam", "rm", &id, "--yes"]).ok();
    assert_eq!(ctx.run(&["exam", "show", &id]).code, 3);
}

//! Abnahmetests der Kommandozeile.
//!
//! Geprüft wird, was ein Benutzer sieht: Ausgabe auf stdout, Meldung auf
//! stderr, Rückgabewert. Die fachlichen Regeln stehen in den Tests von
//! `teacher-class-core` — hier geht es nur ums Durchreichen und ums Format.
//!
//! # Schnappschüsse
//!
//! Die erwartete Ausgabe steht in `tests/snapshots/`. Ändert sich das Format
//! absichtlich, wird nicht der Test von Hand nachgezogen, sondern:
//!
//! ```text
//! just snapshots        # cargo insta review — Änderung ansehen und annehmen
//! ```
//!
//! Kennungen und Zeitstempel sind bei jedem Lauf andere. Sie werden vor dem
//! Vergleich ersetzt (siehe `FILTERS`) — sonst schlüge jeder Lauf fehl.

use std::process::Command;

use tempfile::TempDir;

/// Was in jedem Schnappschuss vereinheitlicht wird.
const FILTERS: &[(&str, &str)] = &[
    // Vollständige UUID zuerst, sonst frisst die Kurzform sie stückweise.
    (
        r"[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}",
        "[uuid]",
    ),
    (r"\b[0-9a-f]{8}\b", "[id]"),
    (r"\d{2}\.\d{2}\.\d{4} \d{2}:\d{2}", "[datum]"),
    (r"\d{4}-\d{2}-\d{2}T[0-9:.]+Z", "[zeit]"),
    // Wegwerfverzeichnisse heißen bei jedem Lauf anders — auf Windows mit
    // Backslashes und Laufwerksbuchstaben.
    (r#"[/\\][^\s"]*[/\\]teacher-class-test-[^\s"]*"#, "[tmp]"),
    // Auf Windows heißt die Datei `teacher-class.exe`, und clap nimmt die Zeile
    // „Usage:" aus argv[0] — nicht aus `#[command(name = ...)]`. Das ist
    // kein Unterschied im Verhalten, nur im Dateinamen. Ohne diese Zeile
    // gölte jeder Schnappschuss mit Hilfetext nur auf zwei von drei
    // Systemen.
    (r"teacher-class\.exe", "teacher-class"),
];

struct Ctx {
    _dir: TempDir,
    data: std::path::PathBuf,
    config: std::path::PathBuf,
}

impl Ctx {
    fn new() -> Self {
        let dir = tempfile::Builder::new()
            .prefix("teacher-class-test-")
            .tempdir()
            .unwrap();
        Self {
            data: dir.path().join("data"),
            config: dir.path().join("config"),
            _dir: dir,
        }
    }

    fn run(&self, args: &[&str]) -> Output {
        // CARGO_BIN_EXE_* setzt cargo für Abnahmetests. Damit wird genau das
        // Binary getestet, das gerade gebaut wurde.
        let out = Command::new(env!("CARGO_BIN_EXE_teacher-class"))
            .args(args)
            .arg("--data-dir")
            .arg(&self.data)
            .arg("--config-dir")
            .arg(&self.config)
            // Ohne stdin ist kein Terminal da — die Rückfrage von `note rm`
            // wird damit zum Fehler, wie im Skript auch.
            .stdin(std::process::Stdio::null())
            .output()
            .expect("teacher-class startbar");

        Output {
            stdout: String::from_utf8(out.stdout).expect("stdout ist UTF-8"),
            stderr: String::from_utf8(out.stderr).expect("stderr ist UTF-8"),
            code: out.status.code().expect("kein Signal"),
        }
    }

    /// Legt Beispieldaten an und gibt die Kurzkennung der Klasse `7a (2025)`
    /// zurück.
    fn seed(&self) -> String {
        self.run(&["class", "add", "8b", "2024"]).ok();
        let out = self.run(&["class", "add", "7a", "2025"]).ok();
        let id = out
            .stdout
            .split_whitespace()
            .nth(1)
            .expect("Kurzkennung in der Rückmeldung")
            .to_string();
        self.run(&[
            "student",
            "add",
            &id,
            "Muster, Max\nBeispiel, Eva\nAbel, Zoe",
        ])
        .ok();
        id
    }
}

struct Output {
    stdout: String,
    stderr: String,
    code: i32,
}

impl Output {
    /// Besteht auf Rückgabewert 0 und reicht die Ausgabe weiter.
    fn ok(self) -> Self {
        assert_eq!(
            self.code, 0,
            "erwartet Rückgabewert 0, bekam {}:\n{}",
            self.code, self.stderr
        );
        self
    }
}

fn snapshot(name: &str, text: &str) {
    // Leerzeichen am Zeilenende (clap setzt sie in Hilfetexten) trägt kein
    // Editor und kein Formatierer zuverlässig mit — sie zählen nicht.
    let text = text
        .lines()
        .map(str::trim_end)
        .collect::<Vec<_>>()
        .join("\n");
    insta::with_settings!({filters => FILTERS.to_vec()}, {
        insta::assert_snapshot!(name, text);
    });
}

// --- Ausgabeformat ---------------------------------------------------------

#[test]
fn hilfe_nennt_alle_befehle() {
    let ctx = Ctx::new();
    snapshot("hilfe", &ctx.run(&["--help"]).ok().stdout);
}

#[test]
fn leere_sammlung_sagt_wie_es_weitergeht() {
    let ctx = Ctx::new();
    snapshot("liste_leer", &ctx.run(&["class", "list"]).ok().stdout);
}

#[test]
fn klassenliste_ist_eine_tabelle() {
    let ctx = Ctx::new();
    ctx.seed();
    snapshot("liste", &ctx.run(&["class", "list"]).ok().stdout);
}

#[test]
fn klassenliste_als_json_ist_ein_array() {
    let ctx = Ctx::new();
    ctx.seed();
    let out = ctx.run(&["class", "list", "--json"]).ok().stdout;
    // Ein Array, damit `jq '.[].name'` ohne Umweg funktioniert.
    assert!(out.trim_start().starts_with('['), "kein Array:\n{out}");
    snapshot("liste_json", &out);
}

#[test]
fn schuelerliste_ist_alphabetisch() {
    let ctx = Ctx::new();
    let id = ctx.seed();
    snapshot("schueler", &ctx.run(&["student", "list", &id]).ok().stdout);
}

// --- Rückgabewerte ---------------------------------------------------------

#[test]
fn ungueltige_eingabe_ergibt_vier() {
    let ctx = Ctx::new();
    let out = ctx.run(&["class", "add", "7 a", "2025"]);
    assert_eq!(out.code, 4);
    assert!(out.stdout.is_empty(), "Fehler gehören nach stderr");
    snapshot("fehler_eingabe", &out.stderr);
}

#[test]
fn eingabefehler_als_json_nennt_das_feld() {
    let ctx = Ctx::new();
    let out = ctx.run(&["class", "add", "7 a", "99", "--json"]);
    assert_eq!(out.code, 4);
    snapshot("fehler_eingabe_json", &out.stderr);
}

#[test]
fn schuelerfehler_nennt_die_zeilennummer() {
    let ctx = Ctx::new();
    let id = ctx.seed();
    let out = ctx.run(&["student", "add", &id, "Gut, Gerd\nohne Komma"]);
    assert_eq!(out.code, 4);
    assert!(out.stderr.contains("Zeile 2"), "{}", out.stderr);
    // Alles oder nichts: auch die gültige Zeile 1 wurde nicht gespeichert.
    let list = ctx.run(&["student", "list", &id, "--json"]).ok().stdout;
    assert!(!list.contains("Gerd"), "{list}");
}

#[test]
fn unbekannte_kennung_ergibt_drei() {
    let ctx = Ctx::new();
    assert_eq!(ctx.run(&["student", "list", "ffffffff"]).code, 3);
}

#[test]
fn falscher_aufruf_ergibt_zwei() {
    // Den Rückgabewert 2 vergibt clap selbst. Der Test hält fest, dass wir ihn
    // nicht versehentlich überschreiben.
    let ctx = Ctx::new();
    assert_eq!(ctx.run(&["class", "gibtsnicht"]).code, 2);
}

#[test]
fn mehrdeutige_kennung_ergibt_fuenf_und_zeigt_die_treffer() {
    let ctx = Ctx::new();

    // Ein Hexzeichen hat 16 mögliche Werte. Bei 17 Klassen müssen sich nach
    // dem Schubfachprinzip zwei das letzte Zeichen teilen — damit ist der
    // mehrdeutige Fall garantiert und der Test nicht vom Zufall abhängig.
    for i in 0..17 {
        ctx.run(&["class", "add", &format!("K{i}"), "2025"]).ok();
    }

    let list: serde_json::Value =
        serde_json::from_str(&ctx.run(&["class", "list", "--json"]).ok().stdout).unwrap();

    let mut nach_endzeichen: std::collections::HashMap<char, usize> = Default::default();
    for class in list.as_array().unwrap() {
        let last = class["id"].as_str().unwrap().chars().next_back().unwrap();
        *nach_endzeichen.entry(last).or_default() += 1;
    }
    let (zeichen, _) = nach_endzeichen
        .iter()
        .find(|(_, n)| **n > 1)
        .expect("Schubfachprinzip garantiert eine Dopplung");

    let out = ctx.run(&["student", "list", &zeichen.to_string()]);
    assert_eq!(out.code, 5, "{}", out.stderr);
    assert!(
        out.stderr.contains("mehr Zeichen angeben"),
        "{}",
        out.stderr
    );
}

#[test]
fn loeschen_ohne_terminal_verlangt_ausdrueckliche_bestaetigung() {
    // Stillschweigend löschen, weil niemand antworten kann, wäre die falsche
    // Voreinstellung.
    let ctx = Ctx::new();
    let id = ctx.seed();
    let out = ctx.run(&["class", "rm", &id]);
    assert_ne!(out.code, 0);
    assert!(out.stderr.contains("--yes"), "{}", out.stderr);
    ctx.run(&["student", "list", &id]).ok();
}

// --- Verhalten -------------------------------------------------------------

#[test]
fn edit_laesst_nicht_genannte_felder_stehen() {
    let ctx = Ctx::new();
    let id = ctx.seed();
    ctx.run(&["class", "edit", &id, "--name", "7c"]).ok();

    let out = ctx.run(&["class", "list", "--json"]).ok().stdout;
    let list: serde_json::Value = serde_json::from_str(&out).unwrap();
    let class = list
        .as_array()
        .unwrap()
        .iter()
        .find(|c| c["name"] == "7c")
        .expect("umbenannte Klasse");
    assert_eq!(class["year"], 2025, "Jahr blieb unverändert");
    assert_eq!(class["student_count"], 3);
}

#[test]
fn eine_klasse_zu_loeschen_nimmt_die_schueler_mit() {
    let ctx = Ctx::new();
    let id = ctx.seed();
    ctx.run(&["class", "rm", &id, "--yes"]).ok();
    assert_eq!(ctx.run(&["student", "list", &id]).code, 3);
}

#[test]
fn einstellungen_ueberleben_den_programmstart() {
    let ctx = Ctx::new();
    ctx.run(&["config", "set", "theme", "dark"]).ok();

    let out = ctx.run(&["config", "show", "--json"]).ok().stdout;
    let config: serde_json::Value = serde_json::from_str(&out).unwrap();
    assert_eq!(config["theme"], "dark");
}

#[test]
fn eine_kaputte_einstellung_wird_nicht_uebernommen() {
    let ctx = Ctx::new();
    ctx.run(&["config", "set", "app_name", "Vereinsportal"])
        .ok();
    assert_eq!(ctx.run(&["config", "set", "accent", "blau"]).code, 4);

    let out = ctx.run(&["config", "show", "--json"]).ok().stdout;
    let config: serde_json::Value = serde_json::from_str(&out).unwrap();
    assert_eq!(config["accent"], "#3b6ea5", "alter Wert blieb stehen");
    assert_eq!(config["app_name"], "Vereinsportal");
}

#[test]
fn schuelernamen_kommen_auch_von_stdin() {
    use std::io::Write;
    let ctx = Ctx::new();
    let id = ctx.seed();
    let mut child = Command::new(env!("CARGO_BIN_EXE_teacher-class"))
        .args(["student", "add", &id, "-", "--json"])
        .arg("--data-dir")
        .arg(&ctx.data)
        .arg("--config-dir")
        .arg(&ctx.config)
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .spawn()
        .unwrap();
    child
        .stdin
        .take()
        .unwrap()
        .write_all(b"Pipe, Paula\n")
        .unwrap();
    let out = child.wait_with_output().unwrap();
    assert!(out.status.success());

    let created: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(created[0]["last_name"], "Pipe");
    assert_eq!(created[0]["first_name"], "Paula");
}

#[test]
fn completions_brauchen_keine_datenbank() {
    // `teacher-class completions zsh` muss auch dort laufen, wo das
    // Datenverzeichnis nicht beschreibbar ist — sonst scheitert die
    // Installation über ein Paket.
    let out = Command::new(env!("CARGO_BIN_EXE_teacher-class"))
        .args(["completions", "zsh"])
        .arg("--data-dir")
        .arg("/nicht/beschreibbar")
        .arg("--config-dir")
        .arg("/nicht/beschreibbar")
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(String::from_utf8_lossy(&out.stdout).starts_with("#compdef teacher-class"));
}

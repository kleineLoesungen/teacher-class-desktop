//! Abnahmetests der Kommandozeile.
//!
//! Geprüft wird, was ein Benutzer sieht: Ausgabe auf stdout, Meldung auf
//! stderr, Rückgabewert. Die fachlichen Regeln stehen in den Tests von
//! `starter-core` — hier geht es nur ums Durchreichen und ums Format.
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
    (r#"[/\\][^\s"]*[/\\]starter-test-[^\s"]*"#, "[tmp]"),
    // Auf Windows heißt die Datei `starter.exe`, und clap nimmt die Zeile
    // „Usage:" aus argv[0] — nicht aus `#[command(name = ...)]`. Das ist
    // kein Unterschied im Verhalten, nur im Dateinamen. Ohne diese Zeile
    // gölte jeder Schnappschuss mit Hilfetext nur auf zwei von drei
    // Systemen.
    (r"starter\.exe", "starter"),
];

struct Ctx {
    _dir: TempDir,
    data: std::path::PathBuf,
    config: std::path::PathBuf,
}

impl Ctx {
    fn new() -> Self {
        let dir = tempfile::Builder::new()
            .prefix("starter-test-")
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
        let out = Command::new(env!("CARGO_BIN_EXE_starter"))
            .args(args)
            .arg("--data-dir")
            .arg(&self.data)
            .arg("--config-dir")
            .arg(&self.config)
            // Ohne stdin ist kein Terminal da — die Rückfrage von `note rm`
            // wird damit zum Fehler, wie im Skript auch.
            .stdin(std::process::Stdio::null())
            .output()
            .expect("starter startbar");

        Output {
            stdout: String::from_utf8(out.stdout).expect("stdout ist UTF-8"),
            stderr: String::from_utf8(out.stderr).expect("stderr ist UTF-8"),
            code: out.status.code().expect("kein Signal"),
        }
    }

    /// Legt Beispieldaten an und gibt das Endstück der Kennung zurück.
    fn seed(&self) -> String {
        self.run(&["note", "add", "Einkauf", "--body", "Milch, Brot"])
            .ok();
        let out = self
            .run(&["note", "add", "Bauplan", "--body", "Regal"])
            .ok();
        out.stdout
            .split_whitespace()
            .nth(1)
            .expect("Kurzkennung in der Rückmeldung")
            .to_string()
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
    snapshot("liste_leer", &ctx.run(&["note", "list"]).ok().stdout);
}

#[test]
fn liste_ist_eine_tabelle() {
    let ctx = Ctx::new();
    ctx.seed();
    snapshot("liste", &ctx.run(&["note", "list"]).ok().stdout);
}

#[test]
fn liste_als_json_ist_ein_array() {
    let ctx = Ctx::new();
    ctx.seed();
    let out = ctx.run(&["note", "list", "--json"]).ok().stdout;
    // Ein Array, damit `jq '.[].title'` ohne Umweg funktioniert.
    assert!(out.trim_start().starts_with('['), "kein Array:\n{out}");
    snapshot("liste_json", &out);
}

#[test]
fn einzelansicht_zeigt_die_volle_kennung() {
    let ctx = Ctx::new();
    let id = ctx.seed();
    snapshot("show", &ctx.run(&["note", "show", &id]).ok().stdout);
}

// --- Rückgabewerte ---------------------------------------------------------

#[test]
fn ungueltige_eingabe_ergibt_vier() {
    let ctx = Ctx::new();
    let out = ctx.run(&["note", "add", ""]);
    assert_eq!(out.code, 4);
    assert!(out.stdout.is_empty(), "Fehler gehören nach stderr");
    snapshot("fehler_eingabe", &out.stderr);
}

#[test]
fn eingabefehler_als_json_nennt_das_feld() {
    let ctx = Ctx::new();
    let out = ctx.run(&["note", "add", "", "--json"]);
    assert_eq!(out.code, 4);
    snapshot("fehler_eingabe_json", &out.stderr);
}

#[test]
fn unbekannte_kennung_ergibt_drei() {
    let ctx = Ctx::new();
    assert_eq!(ctx.run(&["note", "show", "ffffffff"]).code, 3);
}

#[test]
fn falscher_aufruf_ergibt_zwei() {
    // Den Rückgabewert 2 vergibt clap selbst. Der Test hält fest, dass wir ihn
    // nicht versehentlich überschreiben.
    let ctx = Ctx::new();
    assert_eq!(ctx.run(&["note", "gibtsnicht"]).code, 2);
}

#[test]
fn mehrdeutige_kennung_ergibt_fuenf_und_zeigt_die_treffer() {
    let ctx = Ctx::new();

    // Ein Hexzeichen hat 16 mögliche Werte. Bei 17 Notizen müssen sich nach
    // dem Schubfachprinzip zwei das letzte Zeichen teilen — damit ist der
    // mehrdeutige Fall garantiert und der Test nicht vom Zufall abhängig.
    for i in 0..17 {
        ctx.run(&["note", "add", &format!("Notiz {i}")]).ok();
    }

    let list: serde_json::Value =
        serde_json::from_str(&ctx.run(&["note", "list", "--json"]).ok().stdout).unwrap();

    let mut nach_endzeichen: std::collections::HashMap<char, usize> = Default::default();
    for note in list.as_array().unwrap() {
        let last = note["id"].as_str().unwrap().chars().next_back().unwrap();
        *nach_endzeichen.entry(last).or_default() += 1;
    }
    let (zeichen, _) = nach_endzeichen
        .iter()
        .find(|(_, n)| **n > 1)
        .expect("Schubfachprinzip garantiert eine Dopplung");

    let out = ctx.run(&["note", "show", &zeichen.to_string()]);
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
    let out = ctx.run(&["note", "rm", &id]);
    assert_ne!(out.code, 0);
    assert!(out.stderr.contains("--yes"), "{}", out.stderr);
    ctx.run(&["note", "show", &id]).ok();
}

// --- Verhalten -------------------------------------------------------------

#[test]
fn edit_laesst_nicht_genannte_felder_stehen() {
    let ctx = Ctx::new();
    let id = ctx.seed();
    ctx.run(&["note", "edit", &id, "--title", "Anderer Titel"])
        .ok();

    let out = ctx.run(&["note", "show", &id, "--json"]).ok().stdout;
    let note: serde_json::Value = serde_json::from_str(&out).unwrap();
    assert_eq!(note["title"], "Anderer Titel");
    assert_eq!(note["body"], "Regal", "Text blieb unverändert");
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
fn text_kommt_auch_von_stdin() {
    use std::io::Write;
    let ctx = Ctx::new();
    let mut child = Command::new(env!("CARGO_BIN_EXE_starter"))
        .args(["note", "add", "Aus der Pipe", "--body", "-", "--json"])
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
        .write_all(b"Inhalt aus der Pipe")
        .unwrap();
    let out = child.wait_with_output().unwrap();
    assert!(out.status.success());

    let note: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(note["body"], "Inhalt aus der Pipe");
}

#[test]
fn completions_brauchen_keine_datenbank() {
    // `starter completions zsh` muss auch dort laufen, wo das
    // Datenverzeichnis nicht beschreibbar ist — sonst scheitert die
    // Installation über ein Paket.
    let out = Command::new(env!("CARGO_BIN_EXE_starter"))
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
    assert!(String::from_utf8_lossy(&out.stdout).starts_with("#compdef starter"));
}

//! Der Wächter über die Schichtregel.
//!
//! Dieser Test hält Architekturentscheidungen fest, die sonst niemand bemerkt,
//! wenn sie verletzt werden. Der Code kompiliert ja trotzdem.
//!
//! ```text
//! core           →  darf nichts von clap, tauri oder stdout wissen
//! cli            →  darf nichts von tauri wissen
//! desktop        →  darf nichts von clap wissen und schreibt kein SQL
//! ui/src         →  nur api.ts spricht mit Rust
//! ui/components  →  zeichnet nur: kein api, kein innerHTML
//! überall        →  keine Plattform-Fallunterscheidung
//! ```
//!
//! **Diesen Test nicht lockern.** Wenn er anschlägt, steht der Code an der
//! falschen Stelle — die Regel ist nicht das Problem. Der einzige zulässige
//! Grund, ihn anzufassen, ist eine neue Kiste oder ein neues Verzeichnis.

use std::path::{Path, PathBuf};

struct Rule {
    /// Verzeichnis relativ zur Wurzel des Arbeitsbereichs.
    dir: &'static str,
    /// Verbotene Zeichenfolge.
    forbidden: &'static str,
    /// Begründung für den Menschen, der den Fehlschlag liest.
    why: &'static str,
    /// Dateien (relativ zur Wurzel), für die die Regel nicht gilt.
    except: &'static [&'static str],
}

const fn rule(dir: &'static str, forbidden: &'static str, why: &'static str) -> Rule {
    Rule {
        dir,
        forbidden,
        why,
        except: &[],
    }
}

/// Die einzige Datei im Frontend, die mit Rust spricht.
const API_TS: &str = "ui/src/api.ts";

const RULES: &[Rule] = &[
    // --- core kennt keine Schale ------------------------------------------
    rule(
        "crates/core/src",
        "clap",
        "Argumentparsen gehört nach crates/cli",
    ),
    rule(
        "crates/core/src",
        "tauri",
        "Fenster und IPC gehören nach crates/desktop",
    ),
    rule(
        "crates/core/src",
        "println!",
        "core schreibt nicht auf stdout — Werte zurückgeben, die Schale gibt aus",
    ),
    rule(
        "crates/core/src",
        "eprintln!",
        "core schreibt nicht auf stderr — Error zurückgeben, die Schale meldet",
    ),
    rule("crates/core/src", "print!", "siehe println!"),
    rule("crates/core/src", "eprint!", "siehe eprintln!"),
    rule(
        "crates/core/src",
        "std::process::exit",
        "core beendet das Programm nicht — Error zurückgeben",
    ),
    rule(
        "crates/core/src",
        "dbg!",
        "dbg! ist zum Wegwerfen, nicht zum Einchecken",
    ),
    // --- die Schalen kennen einander nicht --------------------------------
    rule(
        "crates/cli/src",
        "tauri",
        "die Kommandozeile öffnet kein Fenster",
    ),
    rule(
        "crates/desktop/src",
        "clap",
        "die Desktop-Schale parst keine Argumente",
    ),
    // --- die Desktop-Schale reicht nur durch ------------------------------
    //
    // Ein Tauri-Befehl, der selbst SQL schreibt, ist eine Regel, die der
    // Kommandozeile fehlt. Die Abfrage gehört als Funktion nach core.
    rule(
        "crates/desktop/src",
        "execute(",
        "SQL gehört nach crates/core — der Befehl ruft nur eine Funktion dort",
    ),
    rule(
        "crates/desktop/src",
        "query_row",
        "SQL gehört nach crates/core",
    ),
    rule(
        "crates/desktop/src",
        "prepare(",
        "SQL gehört nach crates/core",
    ),
    rule(
        "crates/desktop/src",
        "Validator",
        "Prüfung gehört nach crates/core",
    ),
    // --- nur api.ts spricht mit Rust --------------------------------------
    //
    // Ein `invoke("note_craete")` mitten in einer Komponente fällt erst zur
    // Laufzeit auf. In api.ts steht jeder Befehl genau einmal, mit Typen, und
    // `ipc_contract.rs` gleicht die Liste mit der Rust-Seite ab.
    Rule {
        dir: "ui/src",
        forbidden: "@tauri-apps/api/",
        why: "IPC und Fenster-API nur über ui/src/api.ts",
        except: &[API_TS],
    },
    Rule {
        dir: "ui/src",
        forbidden: "invoke(",
        why: "IPC nur über ui/src/api.ts",
        except: &[API_TS],
    },
    Rule {
        dir: "ui/src",
        forbidden: "@tauri-apps/plugin-",
        why: "Plugins nur über ui/src/api.ts — dort steht, was die Anwendung darf",
        except: &[API_TS],
    },
    // --- Komponenten zeichnen nur -----------------------------------------
    rule(
        "ui/src/components",
        "/api",
        "eine Komponente ruft keine Befehle — Rückruf nach oben geben, App.tsx ruft",
    ),
    rule(
        "ui/src/components",
        "innerHTML",
        "HTML nur über ui/src/lib/markdown.ts, dort wird bereinigt",
    ),
    rule(
        "ui/src",
        "dangerouslySetInnerHTML",
        "HTML nur über ui/src/lib/markdown.ts, dort wird bereinigt",
    ),
    // --- keine fremden Server ---------------------------------------------
    //
    // Eine Desktop-Anwendung muss ohne Netz starten. Pakete kommen über npm
    // ins Bündel, nicht von einem CDN. Die CSP in tauri.conf.json würde den
    // Aufruf ohnehin sperren — dann nur mit einer leeren Fläche als Hinweis.
    rule(
        "ui",
        "cdn.",
        "Pakete über npm einbinden, nicht von einem CDN",
    ),
    rule(
        "ui",
        "unpkg.com",
        "Pakete über npm einbinden, nicht von einem CDN",
    ),
];

#[test]
fn schichtregel_wird_eingehalten() {
    let mut verstoesse = Vec::new();

    for rule in RULES {
        for file in source_files(&workspace_root().join(rule.dir)) {
            let rel = relative(&file);
            // Tests dürfen alles anfassen — sie ersetzen IPC durch Attrappen.
            if rule.except.contains(&rel.as_str()) || is_ui_test(&rel) {
                continue;
            }
            let text = std::fs::read_to_string(&file).expect("Quelldatei lesbar");
            for (nr, line) in text.lines().enumerate() {
                if code_only(line).contains(rule.forbidden) {
                    verstoesse.push(format!(
                        "  {rel}:{}\n    enthält {:?} — {}\n    > {}",
                        nr + 1,
                        rule.forbidden,
                        rule.why,
                        line.trim()
                    ));
                }
            }
        }
    }

    assert!(
        verstoesse.is_empty(),
        "Die Schichtregel aus CLAUDE.md ist verletzt:\n\n{}\n\n\
         Den Code verschieben, nicht diesen Test ändern.",
        verstoesse.join("\n")
    );
}

/// Farben stehen nur in `ui/src/theme.css`.
///
/// Wer in einer Komponente `#3b6ea5` schreibt, erzeugt eine Farbe, die sich
/// nicht mehr zentral ändern lässt und im dunklen Aussehen falsch ist.
#[test]
fn farben_stehen_nur_in_theme_css() {
    let mut verstoesse = Vec::new();

    for file in source_files(&workspace_root().join("ui/src")) {
        let rel = relative(&file);
        if rel == "ui/src/theme.css" || is_ui_test(&rel) {
            continue;
        }
        let text = std::fs::read_to_string(&file).expect("Quelldatei lesbar");
        for (nr, line) in text.lines().enumerate() {
            let code = code_only(line);
            if has_hex_color(code) || code.contains("rgb(") || code.contains("rgba(") {
                verstoesse.push(format!("  {rel}:{}\n    > {}", nr + 1, line.trim()));
            }
        }
    }

    assert!(
        verstoesse.is_empty(),
        "Farbe außerhalb von ui/src/theme.css:\n\n{}\n\n\
         Eine Variable in theme.css anlegen und hier var(--…) benutzen.",
        verstoesse.join("\n")
    );
}

/// Die Abhängigkeiten in `Cargo.toml` sind der zweite Riegel.
///
/// Der Textscan oben lässt sich mit einem Alias umgehen. Ohne Eintrag in
/// `Cargo.toml` kompiliert aber gar nichts davon.
#[test]
fn abhaengigkeiten_bleiben_getrennt() {
    const DEPS: &[(&str, &str, &str)] = &[
        ("core", "clap", "Argumentparsen gehört nach crates/cli"),
        (
            "core",
            "tauri",
            "Fenster und IPC gehören nach crates/desktop",
        ),
        ("cli", "tauri", "die Kommandozeile öffnet kein Fenster"),
        (
            "desktop",
            "clap",
            "die Desktop-Schale parst keine Argumente",
        ),
        (
            "desktop",
            "rusqlite",
            "die Desktop-Schale schreibt kein SQL — das tut core",
        ),
    ];

    let mut verstoesse = Vec::new();
    for (crate_name, forbidden, why) in DEPS {
        let manifest = workspace_root()
            .join("crates")
            .join(crate_name)
            .join("Cargo.toml");
        let text = std::fs::read_to_string(&manifest).expect("Cargo.toml lesbar");
        for line in text.lines() {
            let line = line.trim();
            if line.starts_with('#') {
                continue;
            }
            // Ein Abhängigkeitseintrag beginnt mit dem Kistennamen.
            if line.starts_with(forbidden) {
                verstoesse.push(format!(
                    "  crates/{crate_name}/Cargo.toml hängt von {forbidden:?} ab — {why}"
                ));
            }
        }
    }

    assert!(
        verstoesse.is_empty(),
        "Verbotene Abhängigkeit:\n\n{}",
        verstoesse.join("\n")
    );
}

/// Plattform-Fallunterscheidungen gibt es in diesem Kit nicht.
///
/// Das Kit zielt auf macOS, Linux **und** Windows, wird aber fast nur auf
/// macOS entwickelt. Ein `#[cfg(target_os = ...)]` fiele deshalb erst dem
/// Benutzer auf der anderen Plattform auf — oder niemandem.
///
/// Es gibt fast immer einen Weg ohne: `directories` kennt die Verzeichnisse
/// jeder Plattform, und `window.url()` in `ipc_contract.rs` kennt die Adresse
/// der eigenen Oberfläche. Wer doch eine braucht, trägt die Datei unten ein
/// und schreibt dazu, warum — dann steht die Ausnahme an einer Stelle statt
/// verstreut im Quelltext.
const PLATTFORM_CFGS: &[&str] = &[
    "target_os",
    "target_family",
    "cfg(windows)",
    "cfg!(windows)",
    "cfg(unix)",
    "cfg!(unix)",
    "cfg(not(windows))",
    "cfg(not(unix))",
    // Die Bauskript-Variante. Sie ist für ein build.rs die technisch
    // richtige Schreibweise — durchrutschen soll sie trotzdem nicht.
    "CARGO_CFG_TARGET_OS",
    "CARGO_CFG_TARGET_ENV",
];

/// Dateien, die über Plattform-cfgs reden dürfen, ohne welche zu enthalten.
const PLATTFORM_AUSNAHMEN: &[&str] = &[
    // Diese Datei nennt die verbotenen Zeichenfolgen, um nach ihnen zu suchen.
    "crates/core/tests/layering.rs",
    // Hängt das Windows-Anwendungsmanifest an alle Ziele, weil tauri-build
    // es nur an Binärziele hängt. Ohne das stirbt JEDES Testbinär auf
    // Windows beim Laden (STATUS_ENTRYPOINT_NOT_FOUND). `/MANIFEST:EMBED`
    // ist ein Schalter des MSVC-Linkers und hat auf den anderen Systemen
    // keine Entsprechung — hier gibt es keinen Weg ohne.
    "crates/desktop/build.rs",
];

#[test]
fn es_gibt_keine_plattform_fallunterscheidung() {
    let mut verstoesse = Vec::new();

    for file in source_files(&workspace_root().join("crates")) {
        let rel = relative(&file);
        if !rel.ends_with(".rs") || PLATTFORM_AUSNAHMEN.contains(&rel.as_str()) {
            continue;
        }
        let text = std::fs::read_to_string(&file).expect("Quelldatei lesbar");
        for (nr, line) in text.lines().enumerate() {
            let code = code_only(line);
            for needle in PLATTFORM_CFGS {
                if code.contains(needle) {
                    verstoesse.push(format!(
                        "  {rel}:{}\n    enthält {needle:?}\n    > {}",
                        nr + 1,
                        line.trim()
                    ));
                }
            }
        }
    }

    assert!(
        verstoesse.is_empty(),
        "Plattform-Fallunterscheidung im Quelltext:\n\n{}\n\n\
         Gibt es wirklich keinen Weg ohne? `directories` kennt die Pfade jeder\n\
         Plattform, `window.url()` die Adresse der Oberfläche. Wenn doch nicht:\n\
         Datei in PLATTFORM_AUSNAHMEN eintragen, mit Begründung.",
        verstoesse.join("\n")
    );
}

#[test]
fn der_waechter_vergleicht_pfade_plattformunabhaengig() {
    // Die Windows-Probe, die sich auf macOS ausführen lässt.
    assert_eq!(to_slash(r"ui\src\api.ts".to_string()), "ui/src/api.ts");
    assert_eq!(to_slash("ui/src/api.ts".to_string()), "ui/src/api.ts");
    assert!(is_ui_test(&to_slash(r"ui\src\App.test.tsx".to_string())));
}

#[test]
fn der_waechter_findet_farben() {
    assert!(has_hex_color("color: #3b6ea5;"));
    assert!(has_hex_color("color: #fff;"));
    assert!(!has_hex_color("color: var(--accent);"));
    // Ein Anker in einer Adresse ist keine Farbe.
    assert!(!has_hex_color("href=\"#abschnitt-zwei\""));
}

// --- Hilfsmittel -----------------------------------------------------------

fn workspace_root() -> PathBuf {
    // CARGO_MANIFEST_DIR ist crates/core.
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .expect("crates/core liegt zwei Ebenen unter der Wurzel")
        .to_path_buf()
}

/// Der Pfad unterhalb der Wurzel, immer mit `/` als Trennzeichen.
fn relative(p: &Path) -> String {
    let p = p.strip_prefix(workspace_root()).unwrap_or(p);
    to_slash(p.display().to_string())
}

/// Backslashes zu Schrägstrichen.
///
/// Keine Kosmetik: Auf Windows liefert `display()` `ui\\src\\api.ts`, und damit
/// greift keine einzige Ausnahme in `RULES` mehr — `api.ts` und `theme.css`
/// würden sich selbst anzeigen. Ein Wächter, der nur auf einer Plattform
/// richtig urteilt, ist schlimmer als keiner.
fn to_slash(path: String) -> String {
    path.replace('\\', "/")
}

/// Rust-, TypeScript-, CSS- und HTML-Dateien unterhalb von `dir`.
fn source_files(dir: &Path) -> Vec<PathBuf> {
    let mut out = Vec::new();
    let Ok(entries) = std::fs::read_dir(dir) else {
        return out; // Verzeichnis gibt es (noch) nicht — nichts zu prüfen.
    };
    for entry in entries.flatten() {
        let path = entry.path();
        let name = entry.file_name();
        if path.is_dir() {
            // Gebautes und Heruntergeladenes gehört nicht zum Quelltext.
            if name != "node_modules" && name != "dist" {
                out.extend(source_files(&path));
            }
        } else if path
            .extension()
            .is_some_and(|e| ["rs", "ts", "tsx", "css", "html"].iter().any(|x| e == *x))
        {
            out.push(path);
        }
    }
    out.sort();
    out
}

/// Kommentare abschneiden, damit die Doku über die Regel die Regel nicht bricht.
///
/// Absichtlich einfach gehalten: ein `//` innerhalb einer Zeichenkette würde
/// den Rest der Zeile verschlucken. Das Ergebnis wäre ein übersehener Verstoß,
/// kein falscher Alarm.
fn code_only(line: &str) -> &str {
    let trimmed = line.trim_start();
    if trimmed.starts_with("//") || trimmed.starts_with('*') || trimmed.starts_with("/*") {
        return "";
    }
    match line.find("//") {
        Some(i) => &line[..i],
        None => line,
    }
}

fn is_ui_test(rel: &str) -> bool {
    rel.starts_with("ui/") && (rel.contains(".test.") || rel.ends_with("test-setup.ts"))
}

/// `#rgb`, `#rrggbb` oder `#rrggbbaa`, danach kein weiteres Wortzeichen.
fn has_hex_color(line: &str) -> bool {
    let bytes = line.as_bytes();
    for (i, _) in line.match_indices('#') {
        let digits = bytes[i + 1..]
            .iter()
            .take_while(|b| b.is_ascii_hexdigit())
            .count();
        let next = bytes.get(i + 1 + digits);
        let ends_word =
            next.is_none_or(|b| !(b.is_ascii_alphanumeric() || *b == b'-' || *b == b'_'));
        if [3, 6, 8].contains(&digits) && ends_word {
            return true;
        }
    }
    false
}

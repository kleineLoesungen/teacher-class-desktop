//! Die TypeScript-Typen in `ui/src/bindings.ts` entstehen aus den Rust-Typen.
//!
//! Dieser Test erzeugt die Datei neu und vergleicht sie mit der eingecheckten.
//! Weichen sie ab, hat jemand einen Rust-Typ geändert und die Oberfläche nicht
//! nachgezogen — genau der Fehler, der sonst erst zur Laufzeit als `undefined`
//! im Fenster auffällt.
//!
//! ```text
//! just bindings        # Datei neu schreiben, danach `just check`
//! ```
//!
//! **Neuer Typ über IPC?** Unten in `TYPES` eintragen, `#[derive(TS)]` am Typ.

use starter_core::note::{Note, NoteInput, Order};
use starter_core::{Config, ErrorCode, FieldError, Theme};
use starter_desktop::ApiError;
use starter_desktop::commands::AppInfo;
use ts_rs::TS;

const TARGET: &str = "ui/src/bindings.ts";
const UPDATE_ENV: &str = "STARTER_UPDATE_BINDINGS";

fn decl<T: TS>(cfg: &ts_rs::Config) -> String {
    let docs = T::docs()
        .map(|d| format!("{}\n", d.trim_end()))
        .unwrap_or_default();
    format!("{docs}export {}\n", T::decl(cfg))
}

fn generate() -> String {
    let cfg = ts_rs::Config::new();
    // Reihenfolge = Reihenfolge in der Datei. Alphabetisch, damit ein neuer
    // Eintrag keine Diskussion über den richtigen Platz auslöst.
    let types = [
        decl::<ApiError>(&cfg),
        decl::<AppInfo>(&cfg),
        decl::<Config>(&cfg),
        decl::<ErrorCode>(&cfg),
        decl::<FieldError>(&cfg),
        decl::<Note>(&cfg),
        decl::<NoteInput>(&cfg),
        decl::<Order>(&cfg),
        decl::<Theme>(&cfg),
    ];

    let mut out = String::from(
        "// ERZEUGT aus den Rust-Typen — nicht von Hand ändern.\n\
         // Neu schreiben mit:  just bindings\n\
         // Quelle: crates/desktop/tests/bindings.rs\n\n",
    );
    out.push_str(&types.join("\n"));
    out
}

#[test]
fn typescript_typen_sind_aktuell() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .expect("crates/desktop liegt zwei Ebenen unter der Wurzel");
    let path = root.join(TARGET);
    let expected = generate();

    if std::env::var_os(UPDATE_ENV).is_some() {
        std::fs::write(&path, &expected).expect("bindings.ts schreibbar");
        return;
    }

    let actual = std::fs::read_to_string(&path).unwrap_or_default();
    assert!(
        actual == expected,
        "{TARGET} passt nicht mehr zu den Rust-Typen.\n\n\
         Neu erzeugen mit:  just bindings\n\
         Danach die Oberfläche an die neuen Typen anpassen — `npm run build:ui` zeigt, wo."
    );
}

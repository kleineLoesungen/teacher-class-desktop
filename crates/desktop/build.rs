// Liest tauri.conf.json und capabilities/, erzeugt die Berechtigungsschemata
// in gen/schemas und bettet Symbol und Fassungsangaben ein. Ohne diese Datei
// kompiliert `tauri::generate_context!` nicht.

fn main() {
    // Das Manifest hängt tauri-build sonst selbst an — aber nur an
    // Binärziele. Deshalb hier abgeschaltet und unten für alle Ziele
    // nachgeholt. Alles andere (Symbol, Fassung, Hersteller) bleibt.
    let attributes = tauri_build::Attributes::new()
        .windows_attributes(tauri_build::WindowsAttributes::new_without_app_manifest());
    tauri_build::try_build(attributes).expect("tauri-build konnte nicht durchlaufen");

    windows_manifest_an_alle_ziele();
}

/// Hängt `windows-app-manifest.xml` an **jedes** gebaute Ziel, auch an Tests.
///
/// `tauri-build` benutzt dafür `rustc-link-arg-bins` — das erreicht nur das
/// Programm. Ein Testbinär bekommt dann kein Manifest, Windows aktiviert
/// Common Controls v6 nicht, und der Prozess stirbt beim Laden mit
/// `STATUS_ENTRYPOINT_NOT_FOUND` (0xc0000139), **bevor ein einziger Test
/// läuft**. Auf macOS und Linux fällt das nie auf.
///
/// Offene Tauri-Ausgabe dazu: tauri-apps/tauri#13419. Solange die nicht
/// geschlossen ist, bleibt diese Funktion nötig.
///
/// Dies ist die einzige Stelle im Projekt, die nach dem Zielsystem fragt. Sie
/// steht deshalb in `PLATTFORM_AUSNAHMEN` in `crates/core/tests/layering.rs`:
/// `/MANIFEST:EMBED` ist ein Schalter des MSVC-Linkers und hat auf den
/// anderen Systemen keine Entsprechung.
fn windows_manifest_an_alle_ziele() {
    // CARGO_CFG_TARGET_* beschreibt das ZIEL. `cfg!(windows)` beschriebe den
    // Rechner, auf dem das Bauskript läuft — beim Kreuzübersetzen das
    // Falsche.
    let ziel_os = std::env::var("CARGO_CFG_TARGET_OS").unwrap_or_default();
    let ziel_env = std::env::var("CARGO_CFG_TARGET_ENV").unwrap_or_default();
    if ziel_os != "windows" || ziel_env != "msvc" {
        return;
    }

    let manifest = std::path::Path::new(
        &std::env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR ist gesetzt"),
    )
    .join("windows-app-manifest.xml");

    println!("cargo:rerun-if-changed=windows-app-manifest.xml");
    // Ohne -bins / -tests am Ende: gilt für alle Ziele. Genau darum geht es.
    println!("cargo:rustc-link-arg=/MANIFEST:EMBED");
    println!("cargo:rustc-link-arg=/MANIFESTINPUT:{}", manifest.display());
}

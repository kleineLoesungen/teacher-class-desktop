# Alle wiederkehrenden Handgriffe an einem Ort.
# `just` ohne Argument zeigt die Liste.
#
# Installation von just:  brew install just   ·   winget install Casey.Just
#                         cargo install just
#
# JEDES REZEPT IST EIN SCHLICHTER BEFEHL. Keine Pipes, kein `rm -rf`, kein
# `VAR=x befehl`, kein `&&`. Der Grund steht eine Zeile tiefer: Auf Windows
# läuft hier `cmd.exe`, auf macOS und Linux `sh`. Alles, was mehr braucht als
# einen Programmaufruf, steht in scripts/tasks.mjs — eine Datei, die überall
# dieselbe ist. Node ist in diesem Kit ohnehin Pflicht.
set windows-shell := ["cmd.exe", "/c"]

_default:
    @just --list --unsorted

# --- Erster Start ----------------------------------------------------------

# Einmalige Einrichtung: npm-Pakete, Werkzeuge, einmal durchbauen.
setup:
    rustup show
    npm ci
    cargo install --locked cargo-deny cargo-insta
    npm run build:ui
    cargo build
    @echo "Fertig. Weiter mit:  just dev   oder   just cli note list"

# --- Entwicklung -----------------------------------------------------------

# Die Desktop-Anwendung starten. Oberfläche lädt bei jeder Änderung neu,
# Rust wird bei jeder Änderung neu gebaut.
dev:
    npm run tauri dev

# Wie `just dev`, aber auf .local/ statt auf den echten Daten.
dev-sandbox:
    node scripts/tasks.mjs dev-sandbox

# Die Kommandozeile aufrufen:  just cli note add "Titel"
cli *ARGS:
    cargo run -q -p teacher-class-cli -- {{ARGS}}

# Wie `just cli`, aber auf .local/ — dieselben Daten wie `just dev-sandbox`.
sandbox *ARGS:
    node scripts/tasks.mjs sandbox {{ARGS}}

# TypeScript-Typen aus den Rust-Typen neu erzeugen (ui/src/bindings.ts).
bindings:
    node scripts/tasks.mjs bindings

# --- Prüfen ----------------------------------------------------------------

# Der Torwächter. Muss grün sein, bevor etwas committet wird.
check: fmt-check lint test deny
    @echo "Alles grün."

# Formatieren: Rust und TypeScript.
fmt:
    cargo fmt --all
    npm run fmt:ui

fmt-check:
    cargo fmt --all --check

# Warnungen sind Fehler — sonst sammeln sie sich an.
# Biome prüft Format und Regeln der Oberfläche, tsc die Typen.
lint:
    cargo clippy --workspace --all-targets -- -D warnings
    npm run lint:ui
    npx tsc --noEmit

# Rust-Tests brauchen ui/dist: tauri::generate_context! bettet es ein.
test:
    npm run build:ui
    cargo test --workspace
    npm run test:ui

# Lizenzen und bekannte Sicherheitslücken der Abhängigkeiten — Rust und npm.
#
# BEWUSST OHNE `--omit=dev`: Entwicklungswerkzeuge landen zwar nicht im
# Bündel, aber `just dev` startet den Vite-Server auf diesem Rechner. Eine
# Lücke darin trifft nicht die Benutzer der App — wohl aber jeden, der
# daran arbeitet. Genau so eine ist hier schon einmal durchgerutscht.
deny:
    cargo deny check
    npm audit --audit-level=high

# Geänderte Schnappschüsse der CLI ansehen und annehmen.
snapshots:
    cargo insta review

# Doppelte Kisten im Abhängigkeitsbaum — erste Anlaufstelle bei "zwei inkompatible Typen mit demselben Namen".
dupes:
    cargo tree --duplicates

# --- Ausliefern ------------------------------------------------------------

# Bündel für diesen Rechner.
#   macOS    .app + .dmg
#   Linux    .deb + .AppImage
#   Windows  .msi + -setup.exe
bundle:
    npm run tauri build

# macOS: ein Bündel für Apple Silicon und Intel.
# Die Ziele stehen nicht in rust-toolchain.toml — sonst lüde jeder Linux- und
# Windows-Rechner zwei Apple-Bibliotheken herunter, die er nie benutzt.
bundle-macos-universal:
    rustup target add aarch64-apple-darwin x86_64-apple-darwin
    npm run tauri build -- --target universal-apple-darwin

# Kommandozeile mit Handbuch und Vervollständigungen als dist/*.tar.gz
package-cli:
    node scripts/package-cli.mjs

# Symbole aus assets/icon.png neu erzeugen (1024×1024, quadratisch).
icons:
    node scripts/tasks.mjs icons

# --- Aufräumen -------------------------------------------------------------

clean:
    node scripts/tasks.mjs clean

# ACHTUNG: löscht die echten lokalen Daten dieser Anwendung.
data-reset:
    node scripts/tasks.mjs data-reset

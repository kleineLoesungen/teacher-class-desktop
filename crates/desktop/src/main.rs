//! Startet die Desktop-Anwendung. Alles Weitere steht in `lib.rs`.
//!
//! Die Zeile darunter ist der einzige Windows-Sonderfall im ganzen Kit — und
//! keine Plattform-Fallunterscheidung im Sinne von
//! `crates/core/tests/layering.rs`: Sie hängt an `debug_assertions`, nicht am
//! Betriebssystem, und auf macOS und Linux tut sie schlicht nichts.
//!
//! Ohne sie öffnet Windows neben dem Fenster ein Konsolenfenster, das bis zum
//! Programmende stehen bleibt. Im Entwicklungsbau bleibt die Konsole
//! absichtlich erhalten — dort steht das Protokoll aus `just dev`.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    teacher_class_desktop::run();
}

//! Was die Unterbefehle tun.
//!
//! Jede Funktion hier ist dünn: Eingabe einsammeln, `teacher_class_core` rufen,
//! Ergebnis an [`crate::output::Out`] geben. **Keine fachliche Regel.** Eine
//! Prüfung, die hier steht, fehlt der Oberfläche.

pub mod class;
pub mod config;
pub mod confirm;
pub mod student;

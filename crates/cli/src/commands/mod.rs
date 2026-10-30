//! Was die Unterbefehle tun.
//!
//! Jede Funktion hier ist dünn: Eingabe einsammeln, `starter_core` rufen,
//! Ergebnis an [`crate::output::Out`] geben. **Keine fachliche Regel.** Eine
//! Prüfung, die hier steht, fehlt der Oberfläche.

pub mod config;
pub mod note;

use std::io::{IsTerminal, Write};

use anyhow::{Context, Result, bail};

/// Rückfrage vor einer nicht umkehrbaren Aktion.
///
/// Ohne Terminal — im Skript, in der CI — gibt es keine Rückfrage, sondern
/// einen Fehler mit dem Hinweis auf `--yes`. Stillschweigend zu löschen, weil
/// niemand antworten kann, wäre die falsche Voreinstellung.
pub fn confirm(question: &str) -> Result<bool> {
    if !std::io::stdin().is_terminal() {
        bail!("{question} Keine Rückfrage möglich (kein Terminal) — mit --yes bestätigen.");
    }
    print!("{question} [j/N] ");
    std::io::stdout().flush().ok();

    let mut answer = String::new();
    std::io::stdin()
        .read_line(&mut answer)
        .context("Antwort lesen")?;
    Ok(matches!(answer.trim().to_lowercase().as_str(), "j" | "ja"))
}

use anyhow::{Result, bail};
use starter_core::{App, Config, Theme};

use crate::args::ConfigCommand;
use crate::output::Out;

pub fn run(cmd: ConfigCommand, app: &mut App, out: &Out) -> Result<()> {
    match cmd {
        ConfigCommand::Show => {
            out.config(app.config());
            Ok(())
        }

        ConfigCommand::Set { key, value } => {
            let mut config = app.config().clone();
            apply(&mut config, &key, &value)?;
            // set_config prüft und speichert. Schlägt die Prüfung fehl, bleibt
            // der alte Stand unverändert.
            app.set_config(config)?;
            out.done(format!("{key} = {value}"), app.config());
            Ok(())
        }

        ConfigCommand::Reset => {
            app.set_config(Config::default())?;
            out.done("Einstellungen zurückgesetzt.", app.config());
            Ok(())
        }
    }
}

/// Setzt genau einen Schlüssel.
///
/// Die Liste der Schlüssel steht hier und in `ConfigCommand::Set`. Wer ein
/// Feld zu [`Config`] hinzufügt, ergänzt beide — der Test
/// `config_kennt_alle_felder` erinnert daran.
fn apply(config: &mut Config, key: &str, value: &str) -> Result<()> {
    match key {
        "app_name" => config.app_name = value.to_string(),
        "accent" => config.accent = value.to_string(),
        "theme" => {
            config.theme = match value {
                "system" => Theme::System,
                "light" => Theme::Light,
                "dark" => Theme::Dark,
                _ => bail!("theme kennt nur: system, light, dark (nicht {value:?})"),
            }
        }
        _ => bail!("unbekannter Schlüssel {key:?} — bekannt sind: app_name, theme, accent"),
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Erinnert daran, `apply` zu erweitern, wenn `Config` ein Feld bekommt.
    ///
    /// Der Vergleich läuft über die serialisierte Fassung, weil es in Rust
    /// keine Feldliste zur Laufzeit gibt.
    #[test]
    fn config_kennt_alle_felder() {
        let value = serde_json::to_value(Config::default()).unwrap();
        let fields: Vec<&str> = value.as_object().unwrap().keys().map(|k| &**k).collect();

        for field in fields {
            let mut config = Config::default();
            // Ein absichtlich ungültiger Wert: Hauptsache, der Schlüssel ist
            // bekannt. „unbekannter Schlüssel" darf nicht kommen.
            let err = apply(&mut config, field, "ungültig").err();
            if let Some(err) = err {
                assert!(
                    !err.to_string().contains("unbekannter Schlüssel"),
                    "apply() kennt das Feld {field:?} nicht"
                );
            }
        }
    }
}

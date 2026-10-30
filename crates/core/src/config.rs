//! Einstellungen als TOML-Datei.
//!
//! Die Datei ist zum Anfassen gedacht — ein Mensch soll sie im Editor öffnen
//! und verstehen können. Deshalb wenige, flache Schlüssel und Kommentare in der
//! Vorlage.
//!
//! `deny_unknown_fields` ist Absicht: ein Tippfehler im Schlüsselnamen soll
//! auffallen und nicht stillschweigend ignoriert werden.

use std::path::Path;

use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::error::{Error, Result, Validator};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(default, deny_unknown_fields)]
pub struct Config {
    /// Fenstertitel und Name in der Oberfläche.
    pub app_name: String,
    /// Hell, dunkel oder der Systemeinstellung folgen.
    pub theme: Theme,
    /// Akzentfarbe als `#rrggbb`. Siehe docs/DESIGN.md.
    pub accent: String,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            app_name: "Starter".to_string(),
            theme: Theme::System,
            accent: "#3b6ea5".to_string(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default, TS)]
#[serde(rename_all = "lowercase")]
pub enum Theme {
    #[default]
    System,
    Light,
    Dark,
}

impl Theme {
    /// Deutscher Text für die Oberfläche.
    pub fn label(self) -> &'static str {
        match self {
            Theme::System => "Systemeinstellung",
            Theme::Light => "Hell",
            Theme::Dark => "Dunkel",
        }
    }

    pub const ALL: [Theme; 3] = [Theme::System, Theme::Light, Theme::Dark];
}

impl Config {
    /// Liest die Datei. Fehlt sie, gilt [`Config::default`] — eine frisch
    /// installierte Anwendung soll ohne Einrichtung starten.
    pub fn load(path: &Path) -> Result<Self> {
        let text = match std::fs::read_to_string(path) {
            Ok(t) => t,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(Self::default()),
            Err(source) => {
                return Err(Error::Io {
                    path: path.to_path_buf(),
                    source,
                });
            }
        };

        let config: Config = toml::from_str(&text).map_err(|source| Error::ConfigParse {
            path: path.to_path_buf(),
            source,
        })?;
        config.validate()?;
        Ok(config)
    }

    /// Schreibt die Datei. Das Verzeichnis muss es schon geben
    /// ([`crate::Paths::ensure`]).
    pub fn save(&self, path: &Path) -> Result<()> {
        self.validate()?;
        let text = toml::to_string_pretty(self).expect("Config ist immer serialisierbar");
        std::fs::write(path, text).map_err(|source| Error::Io {
            path: path.to_path_buf(),
            source,
        })
    }

    pub fn validate(&self) -> Result<()> {
        let mut v = Validator::new();
        v.require(
            !self.app_name.trim().is_empty(),
            "app_name",
            "darf nicht leer sein",
        );
        v.require(
            is_hex_color(&self.accent),
            "accent",
            "muss die Form #rrggbb haben, zum Beispiel #3b6ea5",
        );
        v.finish()
    }

    /// Die Akzentfarbe als Bytes. Nur gültig, wenn `validate` durchläuft.
    pub fn accent_rgb(&self) -> [u8; 3] {
        parse_hex_color(&self.accent).unwrap_or([0x3b, 0x6e, 0xa5])
    }
}

fn is_hex_color(s: &str) -> bool {
    parse_hex_color(s).is_some()
}

fn parse_hex_color(s: &str) -> Option<[u8; 3]> {
    let hex = s.strip_prefix('#')?;
    if hex.len() != 6 || !hex.bytes().all(|b| b.is_ascii_hexdigit()) {
        return None;
    }
    let byte = |i: usize| u8::from_str_radix(&hex[i..i + 2], 16).ok();
    Some([byte(0)?, byte(2)?, byte(4)?])
}

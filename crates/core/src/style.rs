//! Die kleine CSS-Teilmenge für Druckvorlagen.
//!
//! In einer Vorlage steht optional ein Block `<style> … </style>`. Er kennt
//! nur, was das PDF wirklich zeichnen kann — alles andere ist ein Fehler mit
//! Zeilennummer, nie stilles Ignorieren. Wer `colour: red` tippt, soll es vor
//! dem Ausdruck erfahren und nicht erst auf Papier.
//!
//! * **Selektoren:** `body h1 h2 h3 p li table th td img hr`, mit Komma
//!   mehrere. `body` gilt für alles (Schrift, Farbe, Ausrichtung) und legt mit
//!   `margin` den Seitenrand fest.
//! * **Eigenschaften:** `font-size`, `font-weight`, `font-style`, `color`,
//!   `background`, `text-align`, `line-height`, `margin`, `margin-top`,
//!   `margin-bottom`, `padding`, `border`, `width`, `height`.
//! * **Längen:** `pt`, `px`, `mm`, `cm`, `in`; ohne Einheit gilt `pt`. Nur
//!   `width` kennt außerdem `%` (von der Satzbreite).
//! * **Farben:** Namen (`black`, `gray`, `lightgray`, `red`, `blue` …),
//!   `#rgb`, `#rrggbb` und `rgb(r, g, b)`.

/// Eine Farbe als Anteile von 0 bis 1.
pub type Color = (f64, f64, f64);

pub const BLACK: Color = (0.0, 0.0, 0.0);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Align {
    Left,
    Center,
    Right,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Length {
    Pt(f64),
    /// Prozent der Satzbreite.
    Percent(f64),
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Border {
    pub width: f64,
    pub color: Color,
}

/// Was eine Regel setzt; `None` heißt „nicht gesagt“.
#[derive(Debug, Clone, Default)]
struct Props {
    size: Option<f64>,
    bold: Option<bool>,
    italic: Option<bool>,
    color: Option<Color>,
    background: Option<Color>,
    align: Option<Align>,
    line_height: Option<f64>,
    margin_top: Option<f64>,
    margin_bottom: Option<f64>,
    /// Oben, rechts, unten, links; nur bei `body`.
    page_margin: Option<[f64; 4]>,
    padding: Option<f64>,
    /// `Some(None)` = `border: none`.
    border: Option<Option<Border>>,
    width: Option<Length>,
    height: Option<f64>,
}

impl Props {
    /// `other` überschreibt, was es setzt.
    fn merge(&mut self, other: &Props) {
        macro_rules! take {
            ($($f:ident),*) => { $( if other.$f.is_some() { self.$f = other.$f; } )* };
        }
        take!(
            size,
            bold,
            italic,
            color,
            background,
            align,
            line_height,
            margin_top,
            margin_bottom,
            page_margin,
            padding,
            border,
            width,
            height
        );
    }
}

/// Die fertigen Werte für ein Element.
#[derive(Debug, Clone, Copy)]
pub struct Resolved {
    pub size: f64,
    pub bold: bool,
    pub italic: bool,
    pub color: Color,
    pub background: Option<Color>,
    pub align: Align,
    pub line_height: f64,
    pub margin_top: f64,
    pub margin_bottom: f64,
    pub padding: f64,
    pub border: Option<Border>,
    pub width: Option<Length>,
    pub height: Option<f64>,
}

const SELECTORS: &[&str] = &[
    "body", "h1", "h2", "h3", "p", "li", "table", "th", "td", "img", "hr",
];

const PROPERTIES: &str = "font-size, font-weight, font-style, color, background, text-align, \
     line-height, margin, margin-top, margin-bottom, padding, border, width, height";

#[derive(Debug, Clone, Default)]
pub struct Css {
    rules: Vec<(String, Props)>,
}

impl Css {
    /// Liest den Inhalt eines `<style>`-Blocks. `first_line` ist die Zeile in
    /// der Vorlage, in der er beginnt (für die Fehlermeldung). Ein Fehler ist
    /// schon die fertige Meldung.
    pub fn parse(&mut self, source: &str, first_line: usize) -> Result<(), String> {
        let source = strip_comments(source);
        let mut rest = source.as_str();
        let mut line = first_line;
        while !rest.trim().is_empty() {
            let lead = rest.len() - rest.trim_start().len();
            let rule_line = line + rest[..lead].matches('\n').count();
            let Some(open) = rest.find('{') else {
                return Err(format!(
                    "Zeile {rule_line}: im <style>-Block fehlt „{{“ nach „{}“",
                    rest.trim()
                ));
            };
            let head = &rest[..open];
            let Some(close) = rest[open..].find('}') else {
                return Err(format!(
                    "Zeile {rule_line}: im <style>-Block fehlt „}}“ nach „{}“",
                    head.trim()
                ));
            };
            let body = &rest[open + 1..open + close];
            let selectors: Vec<String> = head
                .split(',')
                .map(|s| s.trim().to_ascii_lowercase())
                .collect();
            for s in &selectors {
                if !SELECTORS.contains(&s.as_str()) {
                    return Err(format!(
                        "Zeile {rule_line}: unbekannter Selektor „{s}“ — erlaubt sind {}",
                        SELECTORS.join(", ")
                    ));
                }
            }
            let props = parse_declarations(body, rule_line, &selectors)?;
            for s in selectors {
                self.rules.push((s, props.clone()));
            }
            line += rest[..open + close + 1].matches('\n').count();
            rest = &rest[open + close + 1..];
        }
        Ok(())
    }

    fn collect(&self, tag: &str) -> Props {
        let mut p = Props::default();
        for (selector, props) in &self.rules {
            if selector == tag {
                p.merge(props);
            }
        }
        p
    }

    /// Seitenrand: oben, rechts, unten, links in Punkt.
    pub fn page_margins(&self) -> [f64; 4] {
        self.collect("body").page_margin.unwrap_or([50.0; 4])
    }

    /// Die Werte für `tag`: eingebaute Vorgabe, darüber `body`, darüber die
    /// Regeln für das Element.
    pub fn resolve(&self, tag: &str) -> Resolved {
        let mut base = Props::default();
        base.merge(&self.collect("body"));
        let body = Resolved {
            size: base.size.unwrap_or(10.0),
            bold: base.bold.unwrap_or(false),
            italic: base.italic.unwrap_or(false),
            color: base.color.unwrap_or(BLACK),
            background: None,
            align: base.align.unwrap_or(Align::Left),
            line_height: base.line_height.unwrap_or(1.2),
            margin_top: 0.0,
            margin_bottom: 0.0,
            padding: 0.0,
            border: None,
            width: None,
            height: None,
        };

        let mut p = builtin(tag);
        p.merge(&self.collect(tag));
        // Zellen und Tabelle teilen sich den Rahmen, wenn die Zelle keinen sagt.
        let border = match p.border {
            Some(b) => b,
            None if tag == "th" || tag == "td" => {
                let mut t = builtin("table");
                t.merge(&self.collect("table"));
                t.border.flatten()
            }
            None => None,
        };
        Resolved {
            size: p.size.unwrap_or(body.size),
            bold: p.bold.unwrap_or(body.bold),
            italic: p.italic.unwrap_or(body.italic),
            color: p.color.unwrap_or(body.color),
            background: p.background,
            align: p.align.unwrap_or(body.align),
            line_height: p.line_height.unwrap_or(body.line_height),
            margin_top: p.margin_top.unwrap_or(0.0),
            margin_bottom: p.margin_bottom.unwrap_or(0.0),
            padding: p.padding.unwrap_or(0.0),
            border,
            width: p.width,
            height: p.height,
        }
    }
}

/// Die Vorgaben, wenn eine Vorlage nichts sagt — sie entsprechen dem Aussehen
/// der Standardvorlage.
fn builtin(tag: &str) -> Props {
    let mut p = Props::default();
    match tag {
        "h1" => {
            p.size = Some(16.0);
            p.bold = Some(true);
            p.margin_bottom = Some(6.0);
        }
        "h2" => {
            p.size = Some(13.0);
            p.bold = Some(true);
            p.margin_top = Some(8.0);
            p.margin_bottom = Some(4.0);
        }
        "h3" => {
            p.size = Some(11.0);
            p.bold = Some(true);
            p.margin_top = Some(6.0);
            p.margin_bottom = Some(3.0);
        }
        "p" => p.margin_bottom = Some(4.0),
        "li" => p.margin_bottom = Some(2.0),
        "table" => {
            p.margin_top = Some(4.0);
            p.margin_bottom = Some(8.0);
            p.border = Some(Some(Border {
                width: 0.5,
                color: BLACK,
            }));
        }
        "th" => {
            p.bold = Some(true);
            p.background = Some((0.9, 0.9, 0.9));
            p.padding = Some(3.0);
        }
        "td" => p.padding = Some(3.0),
        "hr" => {
            p.margin_top = Some(6.0);
            p.margin_bottom = Some(6.0);
            p.border = Some(Some(Border {
                width: 0.5,
                color: BLACK,
            }));
        }
        "img" => p.margin_bottom = Some(6.0),
        _ => {}
    }
    p
}

fn strip_comments(s: &str) -> String {
    let mut out = String::new();
    let mut rest = s;
    while let Some(start) = rest.find("/*") {
        out.push_str(&rest[..start]);
        match rest[start + 2..].find("*/") {
            Some(end) => {
                // Zeilenumbrüche behalten, damit die Zeilenzahlen stimmen.
                out.push_str(&"\n".repeat(rest[start..start + 2 + end].matches('\n').count()));
                rest = &rest[start + 2 + end + 2..];
            }
            None => {
                rest = "";
                break;
            }
        }
    }
    out.push_str(rest);
    out
}

fn parse_declarations(body: &str, line: usize, selectors: &[String]) -> Result<Props, String> {
    let mut p = Props::default();
    for decl in body.split(';') {
        let decl = decl.trim();
        if decl.is_empty() {
            continue;
        }
        let Some((name, value)) = decl.split_once(':') else {
            return Err(format!(
                "Zeile {line}: „{decl}“ — erwartet wird „eigenschaft: wert;“"
            ));
        };
        let (name, value) = (name.trim().to_ascii_lowercase(), value.trim());
        let fail = |what: &str| -> String {
            format!("Zeile {line}: bei „{name}“ ist „{value}“ {what}")
        };
        match name.as_str() {
            "font-size" => {
                p.size = Some(
                    length(value)
                        .filter(|v| *v > 0.0)
                        .ok_or_else(|| fail("keine Größe, z. B. 11pt"))?,
                )
            }
            "font-weight" => {
                p.bold = Some(match value {
                    "bold" | "700" | "800" | "900" => true,
                    "normal" | "400" => false,
                    _ => return Err(fail("nicht möglich — erlaubt sind bold und normal")),
                });
            }
            "font-style" => {
                p.italic = Some(match value {
                    "italic" | "oblique" => true,
                    "normal" => false,
                    _ => return Err(fail("nicht möglich — erlaubt sind italic und normal")),
                });
            }
            "color" => p.color = Some(color(value).ok_or_else(|| fail("keine Farbe"))?),
            "background" | "background-color" => {
                p.background = Some(color(value).ok_or_else(|| fail("keine Farbe"))?);
            }
            "text-align" => {
                p.align = Some(match value {
                    "left" => Align::Left,
                    "center" => Align::Center,
                    "right" => Align::Right,
                    _ => return Err(fail("nicht möglich — erlaubt sind left, center und right")),
                });
            }
            "line-height" => {
                p.line_height = Some(
                    value
                        .parse::<f64>()
                        .ok()
                        .filter(|v| (0.8..=4.0).contains(v))
                        .ok_or_else(|| fail("keine Zahl zwischen 0,8 und 4, z. B. 1.3"))?,
                );
            }
            "margin" => {
                let parts: Vec<f64> = value
                    .split_whitespace()
                    .map(length)
                    .collect::<Option<_>>()
                    .ok_or_else(|| fail("keine Länge, z. B. 20mm"))?;
                let [t, r, b, l] = match parts.as_slice() {
                    [a] => [*a; 4],
                    [v, h] => [*v, *h, *v, *h],
                    [t, r, b, l] => [*t, *r, *b, *l],
                    _ => return Err(fail("nicht möglich — ein, zwei oder vier Längen angeben")),
                };
                if selectors.iter().any(|s| s == "body") {
                    p.page_margin = Some([t, r, b, l]);
                }
                p.margin_top = Some(t);
                p.margin_bottom = Some(b);
            }
            "margin-top" => p.margin_top = Some(length(value).ok_or_else(|| fail("keine Länge"))?),
            "margin-bottom" => {
                p.margin_bottom = Some(length(value).ok_or_else(|| fail("keine Länge"))?);
            }
            "padding" => {
                p.padding = Some(
                    length(value.split_whitespace().next().unwrap_or(""))
                        .ok_or_else(|| fail("keine Länge"))?,
                );
            }
            "border" => {
                p.border = Some(border(value).ok_or_else(|| {
                    fail("nicht möglich — „none“ oder z. B. „0.5pt solid black“")
                })?);
            }
            "width" => {
                p.width = Some(match value.strip_suffix('%') {
                    Some(n) => Length::Percent(
                        n.trim()
                            .parse::<f64>()
                            .ok()
                            .filter(|v| *v > 0.0 && *v <= 100.0)
                            .ok_or_else(|| fail("keine Prozentangabe"))?,
                    ),
                    None => Length::Pt(
                        length(value)
                            .filter(|v| *v > 0.0)
                            .ok_or_else(|| fail("keine Länge"))?,
                    ),
                });
            }
            "height" => {
                p.height = Some(
                    length(value)
                        .filter(|v| *v > 0.0)
                        .ok_or_else(|| fail("keine Länge"))?,
                );
            }
            _ => {
                return Err(format!(
                    "Zeile {line}: unbekannte Eigenschaft „{name}“ — erlaubt sind {PROPERTIES}"
                ));
            }
        }
    }
    Ok(p)
}

/// Eine Länge in Punkt.
pub fn length(value: &str) -> Option<f64> {
    let value = value.trim().to_ascii_lowercase();
    let (number, unit) = match value.find(|c: char| c.is_ascii_alphabetic()) {
        Some(i) => value.split_at(i),
        None => (value.as_str(), "pt"),
    };
    let factor = match unit {
        "pt" => 1.0,
        "px" => 0.75,
        "mm" => 72.0 / 25.4,
        "cm" => 72.0 / 2.54,
        "in" => 72.0,
        _ => return None,
    };
    let n: f64 = number.trim().parse().ok()?;
    (n.is_finite() && (0.0..=2000.0).contains(&n)).then_some(n * factor)
}

fn border(value: &str) -> Option<Option<Border>> {
    if value == "none" || value == "0" {
        return Some(None);
    }
    let mut b = Border {
        width: 0.5,
        color: BLACK,
    };
    // Farben dürfen Leerzeichen enthalten (`rgb(0, 0, 0)`); deshalb erst die
    // Klammer herausnehmen.
    let (head, tail) = match value.find("rgb(") {
        Some(i) => (&value[..i], Some(&value[i..])),
        None => (value, None),
    };
    for part in head.split_whitespace() {
        match part {
            "solid" => {}
            "none" => return Some(None),
            _ => {
                if let Some(w) = length(part) {
                    b.width = w;
                } else {
                    b.color = color(part)?;
                }
            }
        }
    }
    if let Some(t) = tail {
        b.color = color(t)?;
    }
    (b.width > 0.0).then_some(Some(b)).or(Some(None))
}

pub fn color(value: &str) -> Option<Color> {
    let v = value.trim().to_ascii_lowercase();
    let grey = |g: f64| Some((g, g, g));
    match v.as_str() {
        "black" => return Some(BLACK),
        "white" => return grey(1.0),
        "gray" | "grey" => return grey(0.5),
        "lightgray" | "lightgrey" => return grey(0.85),
        "darkgray" | "darkgrey" => return grey(0.3),
        "silver" => return grey(0.75),
        "red" => return Some((0.8, 0.0, 0.0)),
        "green" => return Some((0.0, 0.5, 0.0)),
        "blue" => return Some((0.0, 0.0, 0.8)),
        "navy" => return Some((0.0, 0.0, 0.5)),
        "yellow" => return Some((1.0, 0.9, 0.0)),
        "orange" => return Some((1.0, 0.55, 0.0)),
        "purple" => return Some((0.5, 0.0, 0.5)),
        "teal" => return Some((0.0, 0.5, 0.5)),
        "maroon" => return Some((0.5, 0.0, 0.0)),
        _ => {}
    }
    if let Some(hex) = v.strip_prefix('#') {
        if !hex.chars().all(|c| c.is_ascii_hexdigit()) {
            return None;
        }
        let digit = |s: &str| u8::from_str_radix(s, 16).ok().map(|n| f64::from(n) / 255.0);
        return match hex.len() {
            3 => {
                let d = |i: usize| digit(&hex[i..=i].repeat(2));
                Some((d(0)?, d(1)?, d(2)?))
            }
            6 => Some((digit(&hex[0..2])?, digit(&hex[2..4])?, digit(&hex[4..6])?)),
            _ => None,
        };
    }
    let inner = v.strip_prefix("rgb(")?.strip_suffix(')')?;
    let parts: Vec<f64> = inner
        .split(',')
        .map(|n| {
            n.trim()
                .parse::<f64>()
                .ok()
                .filter(|n| (0.0..=255.0).contains(n))
        })
        .collect::<Option<_>>()?;
    match parts.as_slice() {
        [r, g, b] => Some((r / 255.0, g / 255.0, b / 255.0)),
        _ => None,
    }
}

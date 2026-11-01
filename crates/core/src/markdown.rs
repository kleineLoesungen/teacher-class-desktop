//! Die kleine Markdown-Teilmenge für Druckvorlagen.
//!
//! Gelesen wird, was ein PDF ohne Browser zeichnen kann:
//!
//! * Überschriften `#` bis `###` (mehr `#` gelten als `###`)
//! * Absätze; zwei Leerzeichen am Zeilenende oder `<br>` brechen die Zeile
//! * `**fett**` und `*kursiv*`; ein Backslash vor einem Satzzeichen nimmt ihm
//!   die Bedeutung
//! * Listen mit `-`, `*`, `+` oder `1.`
//! * Tabellen mit `|`; die Zahl der Striche in der Trennzeile bestimmt die
//!   relative Breite der Spalte, Doppelpunkte die Ausrichtung
//! * `---` als Trennlinie
//! * Bilder `![Name](datei.png)`, allein in einem Absatz; ein Titel in
//!   Anführungszeichen gibt die Breite an: `![Logo](logo.png "40mm")`
//! * `<!-- seitenumbruch -->` beginnt eine neue Seite; andere Kommentare
//!   `<!-- … -->` werden ignoriert
//!
//! Alles andere erscheint als Text. Rohes HTML gibt es nicht — auch das ist
//! gewollt: Das Ergebnis ist ein PDF, kein Browserfenster.

use crate::style::Align;

/// Zeilenumbruch innerhalb eines Absatzes oder einer Zelle.
pub const BREAK: char = '\u{2028}';
const PAGE_BREAK: char = '\u{c}';

#[derive(Debug, Clone, PartialEq)]
pub struct Span {
    pub text: String,
    pub bold: bool,
    pub italic: bool,
}

pub type Spans = Vec<Span>;

#[derive(Debug, Clone, PartialEq)]
pub enum Block {
    Heading {
        level: u8,
        spans: Spans,
    },
    Paragraph(Spans),
    List {
        ordered: bool,
        items: Vec<Spans>,
    },
    Table {
        aligns: Vec<Option<Align>>,
        weights: Vec<f64>,
        header: Vec<Spans>,
        rows: Vec<Vec<Spans>>,
    },
    Rule,
    Image {
        src: String,
        alt: String,
        width: Option<String>,
    },
    PageBreak,
}

/// Liest den Text. Der Fehler ist eine fertige Meldung.
pub fn parse(source: &str) -> Result<Vec<Block>, String> {
    let text = prepare(source);
    let lines: Vec<&str> = text.lines().collect();
    let mut blocks = Vec::new();
    let mut i = 0;
    while i < lines.len() {
        let line = lines[i];
        let trimmed = line.trim();
        if trimmed.is_empty() {
            i += 1;
        } else if trimmed.starts_with(PAGE_BREAK) {
            blocks.push(Block::PageBreak);
            i += 1;
        } else if let Some((level, rest)) = heading(trimmed) {
            blocks.push(Block::Heading {
                level,
                spans: inline(rest),
            });
            i += 1;
        } else if is_rule(trimmed) {
            blocks.push(Block::Rule);
            i += 1;
        } else if is_table_start(&lines, i) {
            let (block, used) = table(&lines[i..]);
            blocks.push(block);
            i += used;
        } else if list_marker(trimmed).is_some() {
            let (block, used) = list(&lines[i..]);
            blocks.push(block);
            i += used;
        } else {
            let mut para = String::new();
            let mut used = 0;
            while i + used < lines.len() {
                let l = lines[i + used];
                let t = l.trim();
                if used > 0 && (t.is_empty() || starts_block(&lines, i + used)) {
                    break;
                }
                if t.is_empty() {
                    break;
                }
                if !para.is_empty() {
                    para.push(' ');
                }
                para.push_str(t);
                if l.ends_with("  ") {
                    para.push(BREAK);
                }
                used += 1;
            }
            i += used;
            blocks.push(paragraph(&para)?);
        }
    }
    Ok(blocks)
}

/// Kommentare entfernen; der Seitenumbruch wird zu einer eigenen Zeile.
fn prepare(source: &str) -> String {
    let mut out = String::new();
    let mut rest = source;
    while let Some(start) = rest.find("<!--") {
        out.push_str(&rest[..start]);
        let Some(end) = rest[start..].find("-->") else {
            // Unbeendeter Kommentar: der Rest gehört dazu.
            rest = "";
            break;
        };
        let inner = &rest[start + 4..start + end];
        if inner.trim().eq_ignore_ascii_case("seitenumbruch") {
            out.push_str("\n\u{c}\n");
        } else {
            out.push_str(&"\n".repeat(inner.matches('\n').count()));
        }
        rest = &rest[start + end + 3..];
    }
    out.push_str(rest);
    out.replace("<br>", &BREAK.to_string())
        .replace("<br/>", &BREAK.to_string())
        .replace("<br />", &BREAK.to_string())
}

fn heading(line: &str) -> Option<(u8, &str)> {
    let hashes = line.chars().take_while(|c| *c == '#').count();
    if hashes == 0 {
        return None;
    }
    let rest = line[hashes..].strip_prefix(' ')?;
    Some((hashes.min(3) as u8, rest.trim()))
}

fn is_rule(line: &str) -> bool {
    let compact: String = line.chars().filter(|c| !c.is_whitespace()).collect();
    compact.len() >= 3
        && ['-', '*', '_']
            .iter()
            .any(|m| compact.chars().all(|c| c == *m))
}

fn list_marker(line: &str) -> Option<(bool, &str)> {
    for m in ["- ", "* ", "+ "] {
        if let Some(rest) = line.strip_prefix(m) {
            return Some((false, rest));
        }
    }
    let digits = line.chars().take_while(char::is_ascii_digit).count();
    if digits > 0
        && digits <= 3
        && let Some(rest) = line[digits..].strip_prefix(". ")
    {
        return Some((true, rest));
    }
    None
}

fn starts_block(lines: &[&str], i: usize) -> bool {
    let t = lines[i].trim();
    t.starts_with(PAGE_BREAK)
        || heading(t).is_some()
        || is_rule(t)
        || list_marker(t).is_some()
        || is_table_start(lines, i)
}

fn is_table_start(lines: &[&str], i: usize) -> bool {
    lines[i].trim_start().starts_with('|')
        && lines.get(i + 1).is_some_and(|next| {
            let cells = split_cells(next);
            !cells.is_empty() && cells.iter().all(|c| is_separator(c))
        })
}

fn is_separator(cell: &str) -> bool {
    let c = cell.trim();
    let c = c.strip_prefix(':').unwrap_or(c);
    let c = c.strip_suffix(':').unwrap_or(c);
    !c.is_empty() && c.chars().all(|ch| ch == '-')
}

/// Zellen einer Tabellenzeile; `\|` trennt nicht.
fn split_cells(line: &str) -> Vec<String> {
    let line = line.trim();
    let line = line.strip_prefix('|').unwrap_or(line);
    let mut cells = Vec::new();
    let mut cur = String::new();
    let mut chars = line.chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            '\\' => {
                cur.push('\\');
                if let Some(n) = chars.next() {
                    cur.push(n);
                }
            }
            '|' => cells.push(std::mem::take(&mut cur)),
            c => cur.push(c),
        }
    }
    // Eine Zeile ohne abschließenden Strich hat eine letzte Zelle.
    if !cur.trim().is_empty() {
        cells.push(cur);
    }
    cells
}

fn table(lines: &[&str]) -> (Block, usize) {
    let header_cells = split_cells(lines[0]);
    let sep = split_cells(lines[1]);
    let n = header_cells.len().max(1);
    let mut aligns = Vec::new();
    let mut weights = Vec::new();
    for i in 0..n {
        let s = sep.get(i).map_or("---", |s| s.trim());
        let left = s.starts_with(':');
        let right = s.ends_with(':');
        aligns.push(match (left, right) {
            (true, true) => Some(Align::Center),
            (false, true) => Some(Align::Right),
            (true, false) => Some(Align::Left),
            _ => None,
        });
        weights.push(s.chars().filter(|c| *c == '-').count().max(1) as f64);
    }
    let row = |cells: Vec<String>| -> Vec<Spans> {
        (0..n)
            .map(|i| inline(cells.get(i).map_or("", |c| c.trim())))
            .collect()
    };
    let header = row(header_cells);
    let mut rows = Vec::new();
    let mut used = 2;
    while used < lines.len() && lines[used].trim_start().starts_with('|') {
        rows.push(row(split_cells(lines[used])));
        used += 1;
    }
    (
        Block::Table {
            aligns,
            weights,
            header,
            rows,
        },
        used,
    )
}

fn list(lines: &[&str]) -> (Block, usize) {
    let ordered = list_marker(lines[0].trim()).is_some_and(|(o, _)| o);
    let mut items: Vec<String> = Vec::new();
    let mut used = 0;
    while used < lines.len() {
        let t = lines[used].trim();
        if let Some((_, rest)) = list_marker(t) {
            items.push(rest.trim().to_string());
        } else if t.is_empty() || starts_block(lines, used) || items.is_empty() {
            break;
        } else if let Some(last) = items.last_mut() {
            // Fortsetzungszeile des Eintrags.
            last.push(' ');
            last.push_str(t);
        }
        used += 1;
    }
    (
        Block::List {
            ordered,
            items: items.iter().map(|s| inline(s)).collect(),
        },
        used,
    )
}

fn paragraph(text: &str) -> Result<Block, String> {
    let t = text.trim();
    if t.starts_with("![")
        && let Some(img) = image(t)
    {
        return Ok(img);
    }
    if t.contains("![") {
        return Err(format!(
            "Ein Bild muss allein in einem Absatz stehen: „{t}“ — Form ![Name](datei.png)"
        ));
    }
    Ok(Block::Paragraph(inline(t)))
}

/// `![alt](src)` oder `![alt](src "40mm")`, und sonst nichts.
fn image(t: &str) -> Option<Block> {
    let rest = t.strip_prefix("![")?;
    let (alt, rest) = rest.split_once("](")?;
    let inner = rest.strip_suffix(')')?;
    let (src, width) = match inner.split_once(" \"") {
        Some((src, title)) => (
            src.trim(),
            Some(title.strip_suffix('"')?.trim().to_string()),
        ),
        None => (inner.trim(), None),
    };
    if src.is_empty() || src.contains(')') {
        return None;
    }
    Some(Block::Image {
        src: unescape(src),
        alt: unescape(alt),
        width,
    })
}

fn unescape(s: &str) -> String {
    let mut out = String::new();
    let mut chars = s.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '\\' && chars.peek().is_some_and(char::is_ascii_punctuation) {
            if let Some(n) = chars.next() {
                out.push(n);
            }
        } else {
            out.push(c);
        }
    }
    out
}

/// `**fett**`, `*kursiv*` und Backslash-Escapes.
pub fn inline(text: &str) -> Spans {
    let chars: Vec<char> = text.chars().collect();
    let mut spans: Spans = Vec::new();
    let (mut bold, mut italic) = (false, false);
    let mut buf = String::new();
    let flush = |buf: &mut String, spans: &mut Spans, bold: bool, italic: bool| {
        if !buf.is_empty() {
            spans.push(Span {
                text: std::mem::take(buf),
                bold,
                italic,
            });
        }
    };
    let mut i = 0;
    while i < chars.len() {
        let c = chars[i];
        let next = chars.get(i + 1).copied();
        if c == '\\' && next.is_some_and(|n| n.is_ascii_punctuation()) {
            buf.push(chars[i + 1]);
            i += 2;
        } else if c == '*' && next == Some('*') {
            // Öffnen nur, wenn später ein Schluss kommt; sonst ist es Text.
            if bold || has_closing(&chars[i + 2..], true) {
                flush(&mut buf, &mut spans, bold, italic);
                bold = !bold;
            } else {
                buf.push_str("**");
            }
            i += 2;
        } else if c == '*' {
            if italic || has_closing(&chars[i + 1..], false) {
                flush(&mut buf, &mut spans, bold, italic);
                italic = !italic;
            } else {
                buf.push('*');
            }
            i += 1;
        } else {
            buf.push(c);
            i += 1;
        }
    }
    flush(&mut buf, &mut spans, bold, italic);
    spans
}

fn has_closing(rest: &[char], double: bool) -> bool {
    let mut i = 0;
    while i < rest.len() {
        match rest[i] {
            '\\' => i += 2,
            '*' if double && rest.get(i + 1) == Some(&'*') => return true,
            '*' if !double && rest.get(i + 1) != Some(&'*') => return true,
            '*' if !double => i += 2,
            _ => i += 1,
        }
    }
    false
}

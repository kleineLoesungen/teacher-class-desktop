//! PDF-Ausdruck einer Prüfung — ein PDF je Klasse, gesetzt aus einer
//! Druckvorlage (Markdown mit Platzhaltern, siehe [`crate::template`]).
//!
//! Zwei Arten Vorlage: Eine **Schüler-Vorlage** wird je Schüler ausgefüllt,
//! jeder beginnt auf einer neuen Seite. Eine **Klassen-Vorlage** wird einmal
//! ausgefüllt. Die Standardvorlagen sind eingebaut und ergeben das bisherige
//! Aussehen: eine A4-Seite je Schüler mit Tabelle, Gesamtpunkten, Note und
//! Bewertungsmatrix.
//!
//! Die Seitenaufteilung bestimmt dieser Code, nicht ein Tabellenprogramm.
//! Passt eine Tabelle nicht auf eine Seite (lange Begründungen), läuft sie auf
//! der nächsten weiter, mit wiederholtem Tabellenkopf; eine einzelne Zeile
//! bricht nur um, wenn sie länger als eine ganze Seite ist.
//!
//! **Schrift:** die in jedem PDF-Programm eingebauten Times-Schriften (Roman,
//! Bold, Italic, BoldItalic; Base-14, Kodierung WinAnsi). Es wird nichts
//! eingebettet, also liegt auch keine Schriftdatei im Projekt. Zum Umbrechen
//! braucht der Code die Zeichenbreiten; die stehen in [`WIDTHS`] (Adobe-Metrik
//! von Times-Roman). Zeichen außerhalb von WinAnsi erscheinen als „?“.
//!
//! Das PDF ist von Hand geschrieben (Objekte, Querverweistabelle): Es braucht
//! nur Text, Linien, Flächen und Bilder. Gerechnet wird hier nichts — Summen und
//! Note kommen fertig aus [`crate::grading`].

use std::fmt::Write as _;
use std::path::Path;

use uuid::Uuid;

use crate::class;
use crate::error::{Error, Result, Validator};
use crate::exam::{self, Exam};
use crate::grading::{self, StudentResult};
use crate::image::{self, Data, Image};
use crate::markdown::{self, BREAK, Block, Spans};
use crate::style::{Align, Border, Color, Css, Length, Resolved};
use crate::template::{self, Kind, Values, template_error};

const PAGE_W: f64 = 595.0;
const PAGE_H: f64 = 842.0;
/// Mindestens so viel Platz muss der Satzspiegel lassen.
const MIN_BOX: f64 = 120.0;
/// Times-Bold ist etwas breiter als Times-Roman; beim Messen als Zuschlag.
const BOLD_FACTOR: f64 = 1.08;
/// Von der Zeilenoberkante bis zur Grundlinie, als Anteil der Schriftgröße.
const ASCENT: f64 = 0.8;

/// Breiten von Times-Roman für die Zeichen 32 bis 126 (1000 = Schriftgröße).
#[rustfmt::skip]
const WIDTHS: [u16; 95] = [
    250, 333, 408, 500, 500, 833, 778, 333, 333, 333, 500, 564, 250, 333, 250, 278, // ␠ ! " # $ % & ' ( ) * + , - . /
    500, 500, 500, 500, 500, 500, 500, 500, 500, 500, 278, 278, 564, 564, 564, 444, // 0-9 : ; < = > ?
    921, 722, 667, 667, 722, 611, 556, 722, 722, 333, 389, 722, 611, 889, 722, 722, // @ A-O
    556, 722, 667, 556, 611, 722, 722, 944, 722, 722, 611, 333, 278, 333, 469, 500, // P-Z [ \ ] ^ _
    333, 444, 500, 444, 500, 444, 333, 500, 500, 278, 278, 500, 278, 778, 500, 500, // ` a-o
    500, 500, 333, 389, 278, 500, 500, 722, 500, 500, 444, 480, 200, 480, 541,      // p-z { | } ~
];

/// Das Byte in der WinAnsi-Kodierung; `?`, wenn es das Zeichen dort nicht gibt.
fn win_ansi(c: char) -> u8 {
    match c {
        ' '..='~' => c as u8,
        '\u{a0}'..='\u{ff}' => c as u32 as u8,
        '€' => 0x80,
        '„' => 0x84,
        '‘' => 0x91,
        '’' => 0x92,
        '“' => 0x93,
        '”' => 0x94,
        '•' => 0x95,
        '–' => 0x96,
        '—' => 0x97,
        '\t' => b' ',
        _ => b'?',
    }
}

/// Breite eines WinAnsi-Zeichens in Tausendstel der Schriftgröße.
fn width_of(byte: u8) -> f64 {
    let w = match byte {
        32..=126 => WIDTHS[usize::from(byte) - 32],
        0xC0..=0xC5 => 722,        // Ä À Á Â Ã Å
        0xD2..=0xD6 => 722,        // Ö Ò Ó Ô Õ
        0xD9..=0xDC => 722,        // Ü Ù Ú Û
        0xC8..=0xCB => 611,        // É È Ê Ë
        0xE0..=0xE5 => 444,        // ä à á â ã å
        0xE8..=0xEB => 444,        // é è ê ë
        0x84 | 0x93 | 0x94 => 444, // „ “ ”
        0x91 | 0x92 => 333,        // ‘ ’
        0x97 => 1000,              // —
        _ => 500,                  // ö ü ß – € … und der Rest
    };
    f64::from(w) / 1000.0
}

fn text_width(text: &str, size: f64, bold: bool) -> f64 {
    let w: f64 = text.chars().map(|c| width_of(win_ansi(c))).sum::<f64>() * size;
    if bold { w * BOLD_FACTOR } else { w }
}

/// Punkte so, wie man sie in Deutschland schreibt: `2,5`, `4`.
fn points(p: f64) -> String {
    if (p - p.round()).abs() < 1e-9 {
        format!("{}", p.round())
    } else {
        format!("{p:.1}").replace('.', ",")
    }
}

// --- Zeilen umbrechen ---------------------------------------------------------

#[derive(Debug, Clone)]
struct Run {
    text: String,
    bold: bool,
    italic: bool,
}

#[derive(Debug, Clone, Default)]
struct Line {
    runs: Vec<Run>,
    width: f64,
}

enum Tok {
    Word {
        text: String,
        bold: bool,
        italic: bool,
        space_before: bool,
    },
    Break,
}

fn tokens(spans: &Spans, base_bold: bool, base_italic: bool) -> Vec<Tok> {
    let mut out = Vec::new();
    let mut pending_space = false;
    for span in spans {
        let (bold, italic) = (span.bold || base_bold, span.italic || base_italic);
        let mut word = String::new();
        let mut word_space = false;
        let flush = |word: &mut String, space: bool, out: &mut Vec<Tok>| {
            if !word.is_empty() {
                out.push(Tok::Word {
                    text: std::mem::take(word),
                    bold,
                    italic,
                    space_before: space,
                });
            }
        };
        for c in span.text.chars() {
            if c == BREAK {
                flush(&mut word, word_space, &mut out);
                out.push(Tok::Break);
                pending_space = false;
            } else if c.is_whitespace() {
                flush(&mut word, word_space, &mut out);
                pending_space = true;
            } else {
                if word.is_empty() {
                    word_space = pending_space;
                    pending_space = false;
                }
                word.push(c);
            }
        }
        flush(&mut word, word_space, &mut out);
    }
    out
}

/// Bricht Text auf Zeilen von höchstens `max` Punkt Breite. Wörter werden nicht
/// getrennt; ein Wort, das allein nicht passt, bricht an der Grenze.
fn wrap_spans(spans: &Spans, max: f64, size: f64, bold: bool, italic: bool) -> Vec<Line> {
    let mut lines = Vec::new();
    let mut cur = Line::default();
    let push = |cur: &mut Line, text: &str, b: bool, i: bool, space: bool| {
        let t = if space {
            format!(" {text}")
        } else {
            text.to_owned()
        };
        cur.width += text_width(&t, size, b);
        match cur.runs.last_mut() {
            Some(r) if r.bold == b && r.italic == i => r.text.push_str(&t),
            _ => cur.runs.push(Run {
                text: t,
                bold: b,
                italic: i,
            }),
        }
    };
    for tok in tokens(spans, bold, italic) {
        match tok {
            Tok::Break => lines.push(std::mem::take(&mut cur)),
            Tok::Word {
                text,
                bold: b,
                italic: i,
                space_before,
            } => {
                let mut space = space_before && !cur.runs.is_empty();
                let w = text_width(&text, size, b);
                let sw = if space { text_width(" ", size, b) } else { 0.0 };
                if !cur.runs.is_empty() && cur.width + sw + w > max {
                    lines.push(std::mem::take(&mut cur));
                    space = false;
                }
                if w > max {
                    // Das Wort allein ist zu lang: Zeichen für Zeichen.
                    for c in text.chars() {
                        let cw = text_width(&c.to_string(), size, b);
                        if !cur.runs.is_empty() && cur.width + cw > max {
                            lines.push(std::mem::take(&mut cur));
                        }
                        push(&mut cur, &c.to_string(), b, i, false);
                    }
                } else {
                    push(&mut cur, &text, b, i, space);
                }
            }
        }
    }
    lines.push(cur);
    lines
}

// --- Seiten zeichnen -------------------------------------------------------

fn font(bold: bool, italic: bool) -> &'static str {
    match (bold, italic) {
        (false, false) => "F1",
        (true, false) => "F2",
        (false, true) => "F3",
        (true, true) => "F4",
    }
}

/// Eine Tabelle, die gerade gesetzt wird; die Kopfzeile wiederholt sich auf
/// Folgeseiten.
struct TableCtx {
    widths: Vec<f64>,
    aligns: Vec<Option<Align>>,
    header: Vec<Vec<Line>>,
    th: Resolved,
}

struct Doc<'a> {
    pages: Vec<String>,
    page: String,
    y: f64,
    /// Oben, rechts, unten, links.
    margins: [f64; 4],
    dir: &'a Path,
    images: Vec<(String, Image)>,
    table: Option<TableCtx>,
}

impl<'a> Doc<'a> {
    fn new(dir: &'a Path, file: &'a str, margins: [f64; 4]) -> Result<Self> {
        let [t, r, b, l] = margins;
        if PAGE_W - r - l < MIN_BOX || PAGE_H - t - b < MIN_BOX {
            return Err(template_error(format!(
                "{file}: Der Seitenrand (margin bei body) ist so groß, dass kaum Platz bleibt"
            )));
        }
        Ok(Self {
            pages: Vec::new(),
            page: String::new(),
            y: PAGE_H - t,
            margins,
            dir,
            images: Vec::new(),
            table: None,
        })
    }

    fn top(&self) -> f64 {
        PAGE_H - self.margins[0]
    }
    fn bottom(&self) -> f64 {
        self.margins[2]
    }
    fn left(&self) -> f64 {
        self.margins[3]
    }
    fn width(&self) -> f64 {
        PAGE_W - self.margins[1] - self.margins[3]
    }

    fn finish_page(&mut self) {
        if !self.page.is_empty() {
            self.pages.push(std::mem::take(&mut self.page));
        }
    }

    fn new_page(&mut self) {
        self.finish_page();
        self.y = self.top();
    }

    /// Abstand über einem Block; am Seitenanfang entfällt er.
    fn gap_above(&mut self, pt: f64) {
        if self.y < self.top() - 0.01 {
            self.y -= pt;
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn text(&mut self, x: f64, y: f64, text: &str, size: f64, bold: bool, italic: bool, c: Color) {
        if text.is_empty() {
            return;
        }
        let mut hex = String::new();
        for ch in text.chars() {
            let _ = write!(hex, "{:02X}", win_ansi(ch));
        }
        let f = font(bold, italic);
        let _ = writeln!(
            self.page,
            "BT /{f} {size} Tf {:.3} {:.3} {:.3} rg {x:.2} {y:.2} Td <{hex}> Tj ET",
            c.0, c.1, c.2
        );
    }

    fn rect(
        &mut self,
        x: f64,
        y: f64,
        w: f64,
        h: f64,
        fill: Option<Color>,
        border: Option<Border>,
    ) {
        if let Some(c) = fill {
            let _ = writeln!(
                self.page,
                "{:.3} {:.3} {:.3} rg {x:.2} {y:.2} {w:.2} {h:.2} re f",
                c.0, c.1, c.2
            );
        }
        if let Some(b) = border {
            let _ = writeln!(
                self.page,
                "{:.3} {:.3} {:.3} RG {:.2} w {x:.2} {y:.2} {w:.2} {h:.2} re S",
                b.color.0, b.color.1, b.color.2, b.width
            );
        }
    }

    /// Zeichnet eine umbrochene Zeile in einen Kasten `x .. x + width`, dessen
    /// Oberkante `top` ist; die Höhe ergibt sich aus Schriftgröße und Zeilenabstand.
    fn draw_line(
        &mut self,
        line: &Line,
        st: &Resolved,
        align: Align,
        x: f64,
        width: f64,
        top: f64,
    ) {
        let lh = st.size * st.line_height;
        let mut cx = match align {
            Align::Left => x,
            Align::Center => x + (width - line.width) / 2.0,
            Align::Right => x + width - line.width,
        };
        let base = top - (lh - st.size) / 2.0 - st.size * ASCENT;
        for run in &line.runs {
            self.text(cx, base, &run.text, st.size, run.bold, run.italic, st.color);
            cx += text_width(&run.text, st.size, run.bold);
        }
    }

    /// Ein Absatz, eine Überschrift oder ein Listeneintrag. `marker` ist die
    /// Aufzählung, die vor der ersten Zeile steht.
    #[allow(clippy::too_many_arguments)]
    fn text_block(
        &mut self,
        spans: &Spans,
        st: &Resolved,
        x: f64,
        width: f64,
        keep_with_next: bool,
        marker: Option<(&str, f64)>,
    ) {
        let lines = wrap_spans(spans, width, st.size, st.bold, st.italic);
        let lh = st.size * st.line_height;
        self.gap_above(st.margin_top);
        if keep_with_next {
            let need = lines.len() as f64 * lh + 3.0 * lh;
            if self.y - need < self.bottom() && self.y < self.top() - 0.01 {
                self.new_page();
            }
        }
        for (i, line) in lines.iter().enumerate() {
            if self.y - lh < self.bottom() {
                self.new_page();
            }
            let top = self.y;
            if let Some(bg) = st.background {
                self.rect(x, top - lh, width, lh, Some(bg), None);
            }
            if i == 0
                && let Some((m, mx)) = marker
            {
                let base = top - (lh - st.size) / 2.0 - st.size * ASCENT;
                self.text(mx, base, m, st.size, st.bold, st.italic, st.color);
            }
            self.draw_line(line, st, st.align, x, width, top);
            self.y -= lh;
        }
        self.y -= st.margin_bottom;
    }

    // --- Tabellen ---

    fn wrap_cells(cells: &[Spans], widths: &[f64], st: &Resolved) -> Vec<Vec<Line>> {
        cells
            .iter()
            .zip(widths)
            .map(|(spans, w)| {
                wrap_spans(
                    spans,
                    (w - 2.0 * st.padding).max(10.0),
                    st.size,
                    st.bold,
                    st.italic,
                )
            })
            .collect()
    }

    fn draw_row(&mut self, cells: &[Vec<Line>], st: &Resolved) {
        let ctx = self.table.as_ref().expect("Tabelle gesetzt");
        let (widths, aligns) = (ctx.widths.clone(), ctx.aligns.clone());
        let lh = st.size * st.line_height;
        let lines = cells.iter().map(Vec::len).max().unwrap_or(1).max(1);
        let height = lines as f64 * lh + 2.0 * st.padding;
        let top = self.y;
        let mut x = self.left();
        for (i, (w, cell)) in widths.iter().zip(cells).enumerate() {
            self.rect(x, top - height, *w, height, st.background, st.border);
            let align = aligns.get(i).copied().flatten().unwrap_or(st.align);
            for (k, line) in cell.iter().enumerate() {
                let line_top = top - st.padding - k as f64 * lh;
                self.draw_line(
                    line,
                    st,
                    align,
                    x + st.padding,
                    w - 2.0 * st.padding,
                    line_top,
                );
            }
            x += w;
        }
        self.y -= height;
    }

    fn row_height(cells: &[Vec<Line>], st: &Resolved) -> f64 {
        let lines = cells.iter().map(Vec::len).max().unwrap_or(1).max(1);
        lines as f64 * st.size * st.line_height + 2.0 * st.padding
    }

    /// Neue Seite, auf der die Tabellenkopfzeile wiederholt wird.
    fn continue_table_page(&mut self) {
        self.new_page();
        if let Some(ctx) = &self.table {
            let (header, th) = (ctx.header.clone(), ctx.th);
            self.draw_row(&header, &th);
        }
    }

    /// Setzt eine Zeile; wechselt dazu die Seite, wenn sie nicht mehr passt.
    /// Eine Zeile, die auch auf einer leeren Seite nicht Platz hätte, wird
    /// zeilenweise auf mehrere Seiten verteilt.
    fn place_row(&mut self, cells: &[Vec<Line>], st: &Resolved) {
        let lh = st.size * st.line_height;
        let header_h = self
            .table
            .as_ref()
            .map_or(0.0, |t| Self::row_height(&t.header, &t.th));
        let fresh = ((self.top() - self.bottom() - header_h - 2.0 * st.padding) / lh) as usize;
        let mut cells = cells.to_vec();
        let mut moved = false;
        loop {
            let n = cells.iter().map(Vec::len).max().unwrap_or(1).max(1);
            let room = (self.y - self.bottom() - 2.0 * st.padding).max(0.0);
            let mut fit = (room / lh) as usize;
            if fit >= n {
                self.draw_row(&cells, st);
                return;
            }
            if !moved && (n <= fresh || fit == 0) {
                self.continue_table_page();
                moved = true;
                continue;
            }
            // Zu hoch für jede Seite: so viele Zeilen setzen, wie passen.
            fit = fit.max(1);
            let head: Vec<Vec<Line>> = cells
                .iter()
                .map(|c| c.iter().take(fit).cloned().collect())
                .collect();
            for c in &mut cells {
                c.drain(..fit.min(c.len()));
            }
            self.draw_row(&head, st);
            self.continue_table_page();
            moved = true;
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn table(
        &mut self,
        css: &Css,
        aligns: &[Option<Align>],
        weights: &[f64],
        header: &[Spans],
        rows: &[Vec<Spans>],
    ) {
        let (tb, th, td) = (css.resolve("table"), css.resolve("th"), css.resolve("td"));
        let total: f64 = weights.iter().sum();
        let widths: Vec<f64> = weights.iter().map(|w| self.width() * w / total).collect();
        self.gap_above(tb.margin_top);

        let header_cells = Self::wrap_cells(header, &widths, &th);
        let first = rows.first().map(|r| Self::wrap_cells(r, &widths, &td));
        // Kopfzeile und erste Zeile gehören auf dieselbe Seite.
        let need = Self::row_height(&header_cells, &th)
            + first.as_ref().map_or(0.0, |f| Self::row_height(f, &td));
        if self.y - need < self.bottom() && self.y < self.top() - 0.01 {
            self.new_page();
        }
        self.table = Some(TableCtx {
            widths: widths.clone(),
            aligns: aligns.to_vec(),
            header: header_cells.clone(),
            th,
        });
        self.draw_row(&header_cells, &th);
        for (i, row) in rows.iter().enumerate() {
            let cells = if i == 0 {
                first.clone().unwrap_or_default()
            } else {
                Self::wrap_cells(row, &widths, &td)
            };
            self.place_row(&cells, &td);
        }
        self.table = None;
        self.y -= tb.margin_bottom;
    }

    // --- Bilder ---

    fn image_index(&mut self, src: &str) -> std::result::Result<usize, String> {
        if let Some(i) = self.images.iter().position(|(s, _)| s == src) {
            return Ok(i);
        }
        let rel = Path::new(src);
        let plain = rel
            .components()
            .all(|c| matches!(c, std::path::Component::Normal(_)));
        // `:` schließt Adressen (`https://…`) und Laufwerke (`C:\…`) aus.
        if src.is_empty() || !plain || src.contains(':') {
            return Err(format!(
                "Bild „{src}“: erlaubt sind nur Dateien im Vorlagenordner (kein .., kein absoluter Pfad, keine Adresse)"
            ));
        }
        let img = image::load(&self.dir.join(rel))
            .map_err(|m| format!("Bild „{src}“ {m} (gesucht in {})", self.dir.display()))?;
        self.images.push((src.to_owned(), img));
        Ok(self.images.len() - 1)
    }

    fn image(
        &mut self,
        css: &Css,
        src: &str,
        width: Option<&str>,
    ) -> std::result::Result<(), String> {
        let st = css.resolve("img");
        let idx = self.image_index(src)?;
        let (pw, ph) = (
            f64::from(self.images[idx].1.width),
            f64::from(self.images[idx].1.height),
        );
        let avail = self.width();
        let title = match width {
            Some(w) => Some(
                crate::style::length(w)
                    .filter(|v| *v > 0.0)
                    .ok_or_else(|| format!("Bild „{src}“: „{w}“ ist keine Breite, z. B. 40mm"))?,
            ),
            None => None,
        };
        let mut w = title
            .or(match st.width {
                Some(Length::Pt(v)) => Some(v),
                Some(Length::Percent(p)) => Some(avail * p / 100.0),
                None => None,
            })
            .unwrap_or(pw * 0.75)
            .min(avail);
        let mut h = match (st.height, title.or(st.width.map(|_| w))) {
            (Some(h), None) => h,
            _ => w * ph / pw,
        };
        // Höher als der Satzspiegel: verkleinern, im Seitenverhältnis.
        let max_h = self.top() - self.bottom();
        if h > max_h {
            w *= max_h / h;
            h = max_h;
        }
        self.gap_above(st.margin_top);
        if self.y - h < self.bottom() {
            self.new_page();
        }
        let x = match st.align {
            Align::Left => self.left(),
            Align::Center => self.left() + (avail - w) / 2.0,
            Align::Right => self.left() + avail - w,
        };
        let y = self.y - h;
        let _ = writeln!(
            self.page,
            "q {w:.2} 0 0 {h:.2} {x:.2} {y:.2} cm /Im{} Do Q",
            idx + 1
        );
        self.y -= h + st.margin_bottom;
        Ok(())
    }

    // --- Blöcke ---

    fn render(&mut self, css: &Css, blocks: &[Block]) -> std::result::Result<(), String> {
        let (x, width) = (self.left(), self.width());
        for block in blocks {
            match block {
                Block::Heading { level, spans } => {
                    let st = css.resolve(&format!("h{level}"));
                    self.text_block(spans, &st, x, width, true, None);
                }
                Block::Paragraph(spans) => {
                    let st = css.resolve("p");
                    self.text_block(spans, &st, x, width, false, None);
                }
                Block::List { ordered, items } => {
                    let st = css.resolve("li");
                    let indent = 16.0 + st.size;
                    for (i, item) in items.iter().enumerate() {
                        let marker = if *ordered {
                            format!("{}.", i + 1)
                        } else {
                            "\u{2022}".to_owned()
                        };
                        self.text_block(
                            item,
                            &st,
                            x + indent,
                            width - indent,
                            false,
                            Some((marker.as_str(), x + 4.0)),
                        );
                    }
                }
                Block::Table {
                    aligns,
                    weights,
                    header,
                    rows,
                } => self.table(css, aligns, weights, header, rows),
                Block::Rule => {
                    let st = css.resolve("hr");
                    self.gap_above(st.margin_top);
                    if self.y - 1.0 < self.bottom() {
                        self.new_page();
                    }
                    let y = self.y;
                    if let Some(b) = st.border {
                        let _ = writeln!(
                            self.page,
                            "{:.3} {:.3} {:.3} RG {:.2} w {x:.2} {y:.2} m {:.2} {y:.2} l S",
                            b.color.0,
                            b.color.1,
                            b.color.2,
                            b.width,
                            x + width
                        );
                    }
                    self.y -= st.margin_bottom;
                }
                Block::Image { src, width: w, .. } => self.image(css, src, w.as_deref())?,
                Block::PageBreak => self.new_page(),
            }
        }
        Ok(())
    }
}

// --- PDF-Datei ---------------------------------------------------------------

fn push_object(objects: &mut Vec<Vec<u8>>, body: Vec<u8>) -> usize {
    objects.push(body);
    objects.len()
}

fn stream(dict: &str, data: &[u8]) -> Vec<u8> {
    let mut v = format!("<< {dict} /Length {} >>\nstream\n", data.len()).into_bytes();
    v.extend_from_slice(data);
    v.extend_from_slice(b"\nendstream");
    v
}

fn assemble(pages: &[String], images: &[(String, Image)]) -> Vec<u8> {
    // Objekte: 1 Katalog, 2 Seitenbaum, 3 bis 6 Schriften, dann die Bilder
    // (mit Maske je ein Objekt mehr), danach je Seite ein Seitenobjekt und ein
    // Inhaltsstrom.
    let mut objects: Vec<Vec<u8>> = Vec::new();
    let image_objects: usize = images
        .iter()
        .map(|(_, i)| match &i.data {
            Data::Raw { mask: Some(_), .. } => 2,
            _ => 1,
        })
        .sum();
    let first_page = 7 + image_objects;
    let kids: Vec<String> = (0..pages.len())
        .map(|i| format!("{} 0 R", first_page + 2 * i))
        .collect();
    push_object(&mut objects, b"<< /Type /Catalog /Pages 2 0 R >>".to_vec());
    push_object(
        &mut objects,
        format!(
            "<< /Type /Pages /Kids [{}] /Count {} >>",
            kids.join(" "),
            pages.len()
        )
        .into_bytes(),
    );
    for name in [
        "Times-Roman",
        "Times-Bold",
        "Times-Italic",
        "Times-BoldItalic",
    ] {
        push_object(
            &mut objects,
            format!(
                "<< /Type /Font /Subtype /Type1 /BaseFont /{name} /Encoding /WinAnsiEncoding >>"
            )
            .into_bytes(),
        );
    }
    let mut xobjects = String::new();
    for (n, (_, img)) in images.iter().enumerate() {
        let id = objects.len() + 1;
        let _ = write!(xobjects, "/Im{} {id} 0 R ", n + 1);
        let head = format!(
            "/Type /XObject /Subtype /Image /Width {} /Height {} /BitsPerComponent 8",
            img.width, img.height
        );
        match &img.data {
            Data::Jpeg { bytes, color } => {
                push_object(
                    &mut objects,
                    stream(
                        &format!("{head} /ColorSpace /{color} /Filter /DCTDecode"),
                        bytes,
                    ),
                );
            }
            Data::Raw {
                color,
                packed,
                mask,
            } => {
                let smask = mask.as_ref().map(|_| format!(" /SMask {} 0 R", id + 1));
                push_object(
                    &mut objects,
                    stream(
                        &format!(
                            "{head} /ColorSpace /{color} /Filter /FlateDecode{}",
                            smask.unwrap_or_default()
                        ),
                        packed,
                    ),
                );
                if let Some(m) = mask {
                    push_object(
                        &mut objects,
                        stream(
                            &format!(
                                "/Type /XObject /Subtype /Image /Width {} /Height {} \
                                 /ColorSpace /DeviceGray /BitsPerComponent 8 /Filter /FlateDecode",
                                img.width, img.height
                            ),
                            m,
                        ),
                    );
                }
            }
        }
    }
    let xobject_res = if xobjects.is_empty() {
        String::new()
    } else {
        format!(" /XObject << {xobjects}>>")
    };
    for (i, content) in pages.iter().enumerate() {
        push_object(
            &mut objects,
            format!(
                "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 {PAGE_W} {PAGE_H}] \
                 /Resources << /Font << /F1 3 0 R /F2 4 0 R /F3 5 0 R /F4 6 0 R >>{xobject_res} >> \
                 /Contents {} 0 R >>",
                first_page + 2 * i + 1
            )
            .into_bytes(),
        );
        push_object(&mut objects, stream("", content.as_bytes()));
    }

    let mut out = b"%PDF-1.4\n".to_vec();
    let mut offsets = Vec::new();
    for (i, body) in objects.iter().enumerate() {
        offsets.push(out.len());
        out.extend_from_slice(format!("{} 0 obj\n", i + 1).as_bytes());
        out.extend_from_slice(body);
        out.extend_from_slice(b"\nendobj\n");
    }
    let xref = out.len();
    let mut tail = format!("xref\n0 {}\n0000000000 65535 f \n", objects.len() + 1);
    for offset in offsets {
        let _ = writeln!(tail, "{offset:010} 00000 n ");
    }
    let _ = write!(
        tail,
        "trailer\n<< /Size {} /Root 1 0 R >>\nstartxref\n{xref}\n%%EOF\n",
        objects.len() + 1
    );
    out.extend_from_slice(tail.as_bytes());
    out
}

// --- Werte für die Platzhalter -------------------------------------------------

fn today() -> String {
    let d = crate::timestamp::to_local(crate::timestamp::now());
    format!("{:02}.{:02}.{}", d.day(), u8::from(d.month()), d.year())
}

fn student_values(exam: &Exam, res: &StudentResult) -> Values {
    let mut v = Values::new();
    v.set("schueler", res.student.label())
        .set("nachname", res.student.last_name.clone())
        .set("vorname", res.student.first_name.clone())
        .set("gesamtpunkte", points(res.total))
        .set("note", res.grade.clone().unwrap_or_default())
        .set("abschluss_satz", res.remark.clone());
    let groups = exam
        .groups
        .iter()
        .zip(&res.groups)
        .map(|(group, result)| {
            let mut g = Values::new();
            g.set("name", group.name.clone())
                .set("punkte", points(result.points))
                .set("max_punkte", points(group.max_points));
            let items = group
                .items
                .iter()
                .zip(&result.scores)
                .map(|(item, score)| {
                    let mut i = Values::new();
                    i.set("name", item.name.clone())
                        .set("beschreibung", item.description.clone())
                        .set("punkte", score.points.map(points).unwrap_or_default())
                        .set("max_punkte", points(item.max_points))
                        .set("begruendung", score.note.clone());
                    i
                })
                .collect();
            g.set_list("aufgaben", items);
            g
        })
        .collect();
    v.set_list("gruppen", groups);
    v
}

fn common_values(exam: &Exam, label: &str, results: &[StudentResult]) -> Values {
    let mut v = Values::new();
    v.set("pruefung", exam.name.clone())
        .set("klasse", label)
        .set("max_punkte", points(exam.max_points))
        .set("begruendungen", if exam.with_notes { "ja" } else { "" })
        .set("datum", today())
        .set("anzahl_schueler", results.len().to_string());
    let grades = exam
        .grades
        .iter()
        .map(|g| {
            let count = results
                .iter()
                .filter(|r| r.grade.as_deref() == Some(g.name.as_str()))
                .count();
            let mut n = Values::new();
            n.set("name", g.name.clone())
                .set("von", points(g.from_points))
                .set("bis", points(g.to_points))
                .set("anzahl", count.to_string());
            n
        })
        .collect();
    v.set_list("noten", grades);
    v
}

// --- Ausdruck --------------------------------------------------------------

/// Die Vorlagen im Ordner `dir` (legt Ordner und Standardvorlagen bei Bedarf an).
pub fn templates(dir: &Path) -> Result<template::TemplateList> {
    template::list(dir)
}

/// Das fertige PDF der Klasse `class_id` in der Prüfung `exam_id`, gesetzt aus
/// der Vorlage `template_file` (Dateiname im Ordner `dir`, z. B.
/// `print_test_student_standard.md`).
///
/// Ein Fehler in der Vorlage — unbekannter Platzhalter, falsches CSS, fehlendes
/// Bild — bricht ab, bevor irgendetwas geschrieben wird (`Error::Validation`,
/// Feld `template`, die Meldung nennt Datei und Zeile).
pub fn build(
    conn: &rusqlite::Connection,
    dir: &Path,
    exam_id: Uuid,
    class_id: Uuid,
    template_file: &str,
) -> Result<Vec<u8>> {
    let exam = exam::get(conn, exam_id)?;
    let mut v = Validator::new();
    v.require(
        exam.class_ids.contains(&class_id),
        "class",
        "gehört nicht zur Prüfung",
    );
    v.finish()?;
    let cls = class::get(conn, class_id)?;
    let results = grading::results(conn, exam_id, class_id)?;

    // Anlegen der Standarddateien ist Zugabe: Scheitert es (schreibgeschützter
    // Datenträger), druckt die eingebaute Standardvorlage trotzdem.
    let _ = template::ensure(dir);
    let tpl = template::load(dir, template_file)?;
    let compiled = template::compile(&tpl)?;
    let mut css = Css::default();
    for (text, line) in &compiled.styles {
        css.parse(text, *line)
            .map_err(|m| template_error(format!("{template_file}, {m}")))?;
    }
    let fail = |m: String| template_error(format!("{template_file}: {m}"));

    let label = cls.label();
    let mut doc = Doc::new(dir, template_file, css.page_margins())?;
    let mut common = common_values(&exam, &label, &results);

    match compiled.kind() {
        Kind::Student => {
            for res in &results {
                let mut root = common.clone();
                root.extend(student_values(&exam, res));
                let blocks = markdown::parse(&compiled.render(&root)).map_err(fail)?;
                doc.new_page();
                doc.render(&css, &blocks).map_err(fail)?;
            }
            if results.is_empty() {
                let blocks = vec![
                    Block::Heading {
                        level: 1,
                        spans: markdown::inline(&template::escape(&exam.name)),
                    },
                    Block::Paragraph(markdown::inline(&template::escape(&format!(
                        "Klasse: {label}"
                    )))),
                    Block::Paragraph(markdown::inline("Keine Schüler in dieser Klasse.")),
                ];
                doc.render(&css, &blocks).map_err(fail)?;
            }
        }
        Kind::Class => {
            common.set_list(
                "schueler",
                results.iter().map(|r| student_values(&exam, r)).collect(),
            );
            let blocks = markdown::parse(&compiled.render(&common)).map_err(fail)?;
            doc.render(&css, &blocks).map_err(fail)?;
        }
    }
    doc.finish_page();
    Ok(assemble(&doc.pages, &doc.images))
}

/// Schreibt das PDF nach `path` (überschreibt eine vorhandene Datei).
pub fn write_pdf(
    conn: &rusqlite::Connection,
    dir: &Path,
    exam_id: Uuid,
    class_id: Uuid,
    template_file: &str,
    path: &Path,
) -> Result<()> {
    let mut v = Validator::new();
    let is_pdf = path
        .extension()
        .and_then(|e| e.to_str())
        .is_some_and(|e| e.eq_ignore_ascii_case("pdf"));
    v.require(is_pdf, "path", "muss auf .pdf enden");
    v.finish()?;

    let bytes = build(conn, dir, exam_id, class_id, template_file)?;
    std::fs::write(path, bytes).map_err(|source| Error::Io {
        path: path.to_path_buf(),
        source,
    })
}

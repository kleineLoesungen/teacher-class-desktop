//! PDF-Ausdruck einer Prüfung — ein PDF je Klasse, eine Bewertungskachel je
//! Schüler, jede auf einer eigenen Seite (A4 hochkant).
//!
//! Die Seitenaufteilung bestimmt dieser Code, nicht ein Tabellenprogramm:
//! Jeder Schüler beginnt auf einer neuen Seite. Passt seine Kachel nicht auf
//! eine Seite (lange Begründungen), läuft sie auf der nächsten weiter, mit
//! wiederholtem Tabellenkopf; eine einzelne Zeile bricht nur um, wenn sie
//! länger als eine ganze Seite ist.
//!
//! **Schrift:** die in jedem PDF-Programm eingebaute `Times-Roman` und
//! `Times-Bold` (Base-14, Kodierung WinAnsi). Es wird nichts eingebettet, also
//! liegt auch keine Schriftdatei im Projekt. Zum Umbrechen braucht der Code die
//! Zeichenbreiten; die stehen in [`WIDTHS`] (Adobe-Metrik von Times-Roman).
//! Zeichen außerhalb von WinAnsi erscheinen als „?“.
//!
//! Das PDF ist von Hand geschrieben (Objekte, Querverweistabelle), ohne
//! Abhängigkeit: Es braucht nur Text, Linien und Flächen. Gerechnet wird hier
//! nichts — Summen und Note kommen fertig aus [`crate::grading`].

use std::fmt::Write as _;
use std::path::Path;

use uuid::Uuid;

use crate::class;
use crate::error::{Error, Result, Validator};
use crate::exam::{self, Exam};
use crate::grading::{self, StudentResult};

const PAGE_W: f64 = 595.0;
const PAGE_H: f64 = 842.0;
const MARGIN: f64 = 50.0;
const TOP: f64 = PAGE_H - MARGIN;
const BOTTOM: f64 = MARGIN;
const CONTENT_W: f64 = PAGE_W - 2.0 * MARGIN;

const SIZE: f64 = 10.0;
const LEADING: f64 = 12.0;
const PAD: f64 = 3.0;
/// So viele Zeilen passen sicher in eine Zeile der Tabelle auf einer leeren
/// Seite (abzüglich Überschrift und Tabellenkopf).
const FRESH_LINES: usize = ((TOP - BOTTOM - 70.0) / LEADING) as usize - 2;
/// Times-Bold ist etwas breiter als Times-Roman; beim Messen als Zuschlag.
const BOLD_FACTOR: f64 = 1.08;

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

fn text_width(text: &str, bold: bool) -> f64 {
    let w: f64 = text.chars().map(|c| width_of(win_ansi(c))).sum::<f64>() * SIZE;
    if bold { w * BOLD_FACTOR } else { w }
}

/// Bricht `text` auf Zeilen von höchstens `max` Punkt Breite. Wörter werden
/// nicht getrennt; ein Wort, das allein nicht passt, bricht an der Grenze.
fn wrap(text: &str, max: f64, bold: bool) -> Vec<String> {
    let mut lines = Vec::new();
    for paragraph in text.lines() {
        let mut line = String::new();
        for word in paragraph.split_whitespace() {
            let candidate = if line.is_empty() {
                word.to_string()
            } else {
                format!("{line} {word}")
            };
            if text_width(&candidate, bold) <= max {
                line = candidate;
                continue;
            }
            if !line.is_empty() {
                lines.push(std::mem::take(&mut line));
            }
            // Das Wort allein; zu lang? Dann Zeichen für Zeichen.
            for c in word.chars() {
                let mut next = line.clone();
                next.push(c);
                if !line.is_empty() && text_width(&next, bold) > max {
                    lines.push(std::mem::take(&mut line));
                }
                line.push(c);
            }
        }
        lines.push(line);
    }
    if lines.is_empty() {
        lines.push(String::new());
    }
    lines
}

/// Punkte so, wie man sie in Deutschland schreibt: `2,5`, `4`.
fn points(p: f64) -> String {
    if (p - p.round()).abs() < 1e-9 {
        format!("{}", p.round())
    } else {
        format!("{p:.1}").replace('.', ",")
    }
}

// --- Seiten zeichnen -------------------------------------------------------

#[derive(Clone, Copy)]
struct Col {
    width: f64,
    center: bool,
}

#[derive(Clone, Copy, Default)]
struct Style {
    fill: bool,
    bold: bool,
}

struct Layout {
    pages: Vec<String>,
    page: String,
    y: f64,
    cols: Vec<Col>,
    header: Vec<String>,
    /// Überschrift der Folgeseiten.
    running: String,
}

impl Layout {
    fn new() -> Self {
        Self {
            pages: Vec::new(),
            page: String::new(),
            y: TOP,
            cols: Vec::new(),
            header: Vec::new(),
            running: String::new(),
        }
    }

    fn finish_page(&mut self) {
        if !self.page.is_empty() {
            self.pages.push(std::mem::take(&mut self.page));
        }
    }

    fn new_page(&mut self) {
        self.finish_page();
        self.y = TOP;
    }

    fn text(&mut self, x: f64, y: f64, text: &str, size: f64, bold: bool) {
        if text.is_empty() {
            return;
        }
        let mut hex = String::new();
        for c in text.chars() {
            let _ = write!(hex, "{:02X}", win_ansi(c));
        }
        let font = if bold { "F2" } else { "F1" };
        let _ = writeln!(
            self.page,
            "BT /{font} {size} Tf {x:.2} {y:.2} Td <{hex}> Tj ET"
        );
    }

    fn rect(&mut self, x: f64, y: f64, w: f64, h: f64, fill: bool) {
        if fill {
            let _ = writeln!(self.page, "0.9 g {x:.2} {y:.2} {w:.2} {h:.2} re f 0 g");
        }
        let _ = writeln!(self.page, "0.5 w {x:.2} {y:.2} {w:.2} {h:.2} re S");
    }

    /// Eine Zeile Text über die ganze Breite (außerhalb der Tabelle).
    fn line(&mut self, text: &str, size: f64, bold: bool) {
        self.y -= size * 1.3;
        let (x, y) = (MARGIN, self.y);
        self.text(x, y, text, size, bold);
    }

    /// Neue Seite mit Überschrift der Folgeseiten und Tabellenkopf.
    fn continue_page(&mut self) {
        self.new_page();
        let running = self.running.clone();
        self.line(&running, 11.0, true);
        self.y -= 8.0;
        self.draw_header();
    }

    fn draw_header(&mut self) {
        let header = self.header.clone();
        if header.is_empty() {
            return;
        }
        let cells = header.iter().map(|h| vec![h.clone()]).collect();
        self.draw_row(
            cells,
            Style {
                fill: true,
                bold: true,
            },
        );
    }

    /// Zeichnet eine Zeile. `cells[i]` sind die schon umbrochenen Zeilen der
    /// Spalte `i`.
    fn draw_row(&mut self, cells: Vec<Vec<String>>, style: Style) {
        let lines = cells.iter().map(Vec::len).max().unwrap_or(1).max(1);
        let height = lines as f64 * LEADING + 2.0 * PAD;
        let top = self.y;
        let cols = self.cols.clone();
        let mut x = MARGIN;
        for (col, cell) in cols.iter().zip(&cells) {
            self.rect(x, top - height, col.width, height, style.fill);
            for (i, text) in cell.iter().enumerate() {
                let tw = text_width(text, style.bold);
                let tx = if col.center {
                    x + (col.width - tw) / 2.0
                } else {
                    x + PAD
                };
                let ty = top - PAD - (i as f64 + 1.0) * LEADING + 3.0;
                self.text(tx, ty, text, SIZE, style.bold);
            }
            x += col.width;
        }
        self.y -= height;
    }

    /// Setzt eine Zeile; wechselt dazu die Seite, wenn sie nicht mehr passt.
    /// Eine Zeile, die auch auf einer leeren Seite nicht Platz hätte, wird
    /// zeilenweise auf mehrere Seiten verteilt.
    fn place_row(&mut self, cells: &[Vec<String>], style: Style) {
        let mut cells = cells.to_vec();
        loop {
            let n = cells.iter().map(Vec::len).max().unwrap_or(1).max(1);
            let room = (self.y - BOTTOM - 2.0 * PAD).max(0.0);
            let fit = (room / LEADING) as usize;
            if fit >= n {
                self.draw_row(cells, style);
                return;
            }
            if n <= FRESH_LINES || fit == 0 {
                self.continue_page();
                continue;
            }
            // Zu hoch für jede Seite: so viele Zeilen setzen, wie passen.
            let head: Vec<Vec<String>> = cells
                .iter()
                .map(|c| c.iter().take(fit).cloned().collect())
                .collect();
            for c in &mut cells {
                c.drain(..fit.min(c.len()));
            }
            self.draw_row(head, style);
            self.continue_page();
        }
    }

    fn row(&mut self, texts: &[&str], style: Style) {
        let cells: Vec<Vec<String>> = texts
            .iter()
            .zip(&self.cols)
            .map(|(t, col)| wrap(t, col.width - 2.0 * PAD, style.bold))
            .collect();
        self.place_row(&cells, style);
    }
}

// --- Inhalt ----------------------------------------------------------------

fn cols_for(with_notes: bool) -> Vec<Col> {
    let c = |width, center| Col { width, center };
    if with_notes {
        vec![
            c(95.0, false),
            c(140.0, false),
            c(40.0, true),
            c(40.0, true),
            c(CONTENT_W - 315.0, false),
        ]
    } else {
        vec![
            c(130.0, false),
            c(CONTENT_W - 230.0, false),
            c(50.0, true),
            c(50.0, true),
        ]
    }
}

fn write_student(lay: &mut Layout, exam: &Exam, class_label: &str, res: &StudentResult) {
    let sum = Style {
        fill: true,
        bold: true,
    };
    lay.new_page();
    lay.line(&exam.name, 16.0, true);
    lay.y -= 4.0;
    lay.line(&format!("Schüler/in: {}", res.student.label()), 12.0, false);
    lay.line(&format!("Klasse: {class_label}"), 12.0, false);
    lay.y -= 10.0;

    lay.running = format!("{} – {}", exam.name, res.student.label());
    lay.cols = cols_for(exam.with_notes);
    lay.header = if exam.with_notes {
        vec!["Aufgabe", "Beschreibung", "Punkte", "Max.", "Begründung"]
    } else {
        vec!["Aufgabe", "Beschreibung", "Punkte", "Max."]
    }
    .into_iter()
    .map(String::from)
    .collect();
    lay.draw_header();

    for (group, result) in exam.groups.iter().zip(&res.groups) {
        let (got, max) = (points(result.points), points(group.max_points));
        let mut head = vec![group.name.as_str(), "", got.as_str(), max.as_str()];
        if exam.with_notes {
            head.push("");
        }
        // Die Gruppenzeile gehört auf eine Seite mit ihrem ersten Punkt.
        if lay.y - BOTTOM < 4.0 * LEADING + 4.0 * PAD {
            lay.continue_page();
        }
        lay.row(&head, sum);
        for (item, score) in group.items.iter().zip(&result.scores) {
            let p = score.points.map(points).unwrap_or_default();
            let max = points(item.max_points);
            let mut cells = vec![
                item.name.as_str(),
                item.description.as_str(),
                p.as_str(),
                max.as_str(),
            ];
            if exam.with_notes {
                cells.push(score.note.as_str());
            }
            lay.row(&cells, Style::default());
        }
    }

    let total = points(res.total);
    let max = points(exam.max_points);
    let mut totals = vec!["Gesamtpunkte", "", total.as_str(), max.as_str()];
    let grade = res.grade.clone().unwrap_or_default();
    let mut grade_row = vec!["Note", "", grade.as_str(), ""];
    if exam.with_notes {
        totals.push("");
        grade_row.push("");
    }
    lay.row(&totals, sum);
    lay.row(&grade_row, sum);

    // Bewertungsmatrix zur Einordnung.
    if lay.y - BOTTOM < 5.0 * LEADING + 6.0 * PAD {
        lay.continue_page();
    }
    lay.y -= 14.0;
    lay.line("Bewertungsmatrix", 12.0, true);
    lay.y -= 6.0;
    lay.cols = vec![
        Col {
            width: 80.0,
            center: true,
        },
        Col {
            width: 140.0,
            center: true,
        },
    ];
    lay.header = vec!["Note".into(), "Punkte (ab – bis)".into()];
    lay.draw_header();
    for g in &exam.grades {
        let range = format!("{} – {}", points(g.from_points), points(g.to_points));
        lay.row(&[g.name.as_str(), range.as_str()], Style::default());
    }
}

// --- PDF-Datei ---------------------------------------------------------------

fn assemble(pages: &[String]) -> Vec<u8> {
    // Objekte: 1 Katalog, 2 Seitenbaum, 3 und 4 Schriften, danach je Seite
    // ein Seitenobjekt und ein Inhaltsstrom.
    let mut objects: Vec<String> = Vec::new();
    let kids: Vec<String> = (0..pages.len())
        .map(|i| format!("{} 0 R", 5 + 2 * i))
        .collect();
    objects.push("<< /Type /Catalog /Pages 2 0 R >>".into());
    objects.push(format!(
        "<< /Type /Pages /Kids [{}] /Count {} >>",
        kids.join(" "),
        pages.len()
    ));
    for font in ["Times-Roman", "Times-Bold"] {
        objects.push(format!(
            "<< /Type /Font /Subtype /Type1 /BaseFont /{font} /Encoding /WinAnsiEncoding >>"
        ));
    }
    for (i, content) in pages.iter().enumerate() {
        objects.push(format!(
            "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 {PAGE_W} {PAGE_H}] \
             /Resources << /Font << /F1 3 0 R /F2 4 0 R >> >> /Contents {} 0 R >>",
            6 + 2 * i
        ));
        objects.push(format!(
            "<< /Length {} >>\nstream\n{content}endstream",
            content.len()
        ));
    }

    let mut out = String::from("%PDF-1.4\n");
    let mut offsets = Vec::new();
    for (i, body) in objects.iter().enumerate() {
        offsets.push(out.len());
        let _ = write!(out, "{} 0 obj\n{body}\nendobj\n", i + 1);
    }
    let xref = out.len();
    let _ = write!(out, "xref\n0 {}\n0000000000 65535 f \n", objects.len() + 1);
    for offset in offsets {
        let _ = writeln!(out, "{offset:010} 00000 n ");
    }
    let _ = write!(
        out,
        "trailer\n<< /Size {} /Root 1 0 R >>\nstartxref\n{xref}\n%%EOF\n",
        objects.len() + 1
    );
    out.into_bytes()
}

/// Das fertige PDF der Klasse `class_id` in der Prüfung `exam_id`.
pub fn build(conn: &rusqlite::Connection, exam_id: Uuid, class_id: Uuid) -> Result<Vec<u8>> {
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

    let mut lay = Layout::new();
    let label = cls.label();
    for res in &results {
        write_student(&mut lay, &exam, &label, res);
    }
    if results.is_empty() {
        lay.new_page();
        lay.line(&exam.name, 16.0, true);
        lay.line(&format!("Klasse: {label}"), 12.0, false);
        lay.line("Keine Schüler in dieser Klasse.", 12.0, false);
    }
    lay.finish_page();
    Ok(assemble(&lay.pages))
}

/// Schreibt das PDF nach `path` (überschreibt eine vorhandene Datei).
pub fn write_pdf(
    conn: &rusqlite::Connection,
    exam_id: Uuid,
    class_id: Uuid,
    path: &Path,
) -> Result<()> {
    let mut v = Validator::new();
    let is_pdf = path
        .extension()
        .and_then(|e| e.to_str())
        .is_some_and(|e| e.eq_ignore_ascii_case("pdf"));
    v.require(is_pdf, "path", "muss auf .pdf enden");
    v.finish()?;

    let bytes = build(conn, exam_id, class_id)?;
    std::fs::write(path, bytes).map_err(|source| Error::Io {
        path: path.to_path_buf(),
        source,
    })
}

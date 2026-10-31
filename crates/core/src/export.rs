//! Excel-Export einer Prüfung.
//!
//! Die Datei enthält alles, was in der App eingetragen ist (Punkte und
//! Begründungen), und lässt sich in Excel weiter bearbeiten: Summen und Noten
//! rechnen **Formeln**. Ändert die Lehrkraft die Bewertungsmatrix (oder eine
//! Höchstpunktzahl) auf dem Blatt „Bewertung“, ändern sich die Noten.
//!
//! Blätter:
//!
//! * **Bewertung** — Prüfungsschema (Gruppen, Punkte, Beschreibungen,
//!   Höchstpunktzahlen, Summen) und Bewertungsmatrix (Note, ab, bis). „ab“ und
//!   die Höchstpunktzahlen sind Eingaben, alles andere Formeln.
//! * **je Klasse ein Blatt** (Blattname `7a (2025)`). Die Form hängt davon ab,
//!   ob die Prüfung Begründungen hat (`with_notes`):
//!   * ohne: **Matrix** — Schüler in den Zeilen, Punkte des Schemas in den
//!     Spalten, Summe je Gruppe, Gesamtpunkte, Note.
//!   * mit: **eine Bewertungskachel je Schüler** — untereinander, jede
//!     Kachel mit Aufgaben, Beschreibung, Punkten, Höchstpunkten und
//!     Begründung, darunter Summe, Note und Bewertungsmatrix. Vor jeder Kachel
//!     steht ein Seitenumbruch, damit ein Schüler genau eine Seite ergibt
//!     (Hochformat A4, Druckmaßstab fest). Drucken oder als PDF speichern
//!     macht Excel; einzelne Schüler über „Seiten von … bis …“.
//!
//! Die Formeln sind in `crates/core/tests/export.rs` nur als Text geprüft;
//! gerechnet hat sie dort niemand. Ein Blick in Excel oder LibreOffice
//! bleibt nötig.

use std::path::Path;

use rust_xlsxwriter::utility::{row_col_to_cell, row_col_to_cell_absolute};
use rust_xlsxwriter::{
    Color, Format, FormatAlign, FormatBorder, Formula, Workbook, Worksheet, XlsxError,
};
use uuid::Uuid;

use crate::class;
use crate::error::{Error, Result, Validator};
use crate::exam::{self, Exam};
use crate::grading::{self, StudentResult};

/// Blattname der Bewertung.
pub const SHEET_GRADING: &str = "Bewertung";

impl From<XlsxError> for Error {
    fn from(err: XlsxError) -> Self {
        Error::Export(err.to_string())
    }
}

/// Schreibt die Prüfung als `.xlsx` nach `path` (überschreibt eine vorhandene
/// Datei).
pub fn write_xlsx(conn: &rusqlite::Connection, exam_id: Uuid, path: &Path) -> Result<()> {
    let mut v = Validator::new();
    let is_xlsx = path
        .extension()
        .and_then(|e| e.to_str())
        .is_some_and(|e| e.eq_ignore_ascii_case("xlsx"));
    v.require(is_xlsx, "path", "muss auf .xlsx enden");
    v.finish()?;

    let bytes = build(conn, exam_id)?;
    std::fs::write(path, bytes).map_err(|source| Error::Io {
        path: path.to_path_buf(),
        source,
    })
}

/// Die fertige Arbeitsmappe als Bytes.
pub fn build(conn: &rusqlite::Connection, exam_id: Uuid) -> Result<Vec<u8>> {
    let exam = exam::get(conn, exam_id)?;

    let mut classes = Vec::new();
    for id in &exam.class_ids {
        let cls = class::get(conn, *id)?;
        let results = grading::results(conn, exam_id, *id)?;
        classes.push((cls, results));
    }

    let styles = Styles::new();
    let mut grading_sheet = Worksheet::new();
    grading_sheet.set_name(SHEET_GRADING)?;
    let grading_refs = write_grading(&mut grading_sheet, &exam, &classes, &styles)?;

    let mut workbook = Workbook::new();
    workbook.push_worksheet(grading_sheet);
    for (cls, results) in &classes {
        let mut ws = Worksheet::new();
        ws.set_name(cls.label())?;
        if exam.with_notes {
            write_tiles(
                &mut ws,
                &exam,
                &grading_refs,
                &cls.label(),
                results,
                &styles,
            )?;
        } else {
            write_class(&mut ws, &exam, &grading_refs, results, &styles)?;
        }
        workbook.push_worksheet(ws);
    }

    Ok(workbook.save_to_buffer()?)
}

// --- Formate ---------------------------------------------------------------

struct Styles {
    title: Format,
    head: Format,
    label: Format,
    input: Format,
    calc: Format,
    sum: Format,
    /// Mehrzeiliger Text (Beschreibung), oben ausgerichtet.
    wrap: Format,
    /// Eingabe von Text (Begründung): mehrzeilig, linksbündig, oben.
    note: Format,
}

impl Styles {
    fn new() -> Self {
        let light = Color::RGB(0x00EE_EEEE);
        let sum_fill = Color::RGB(0x00F6_F6F6);
        Self {
            title: Format::new().set_bold().set_font_size(14),
            head: Format::new()
                .set_bold()
                .set_background_color(light)
                .set_border(FormatBorder::Thin)
                .set_align(FormatAlign::Center),
            label: Format::new().set_border(FormatBorder::Thin),
            input: Format::new()
                .set_border(FormatBorder::Thin)
                .set_align(FormatAlign::Center),
            calc: Format::new()
                .set_border(FormatBorder::Thin)
                .set_align(FormatAlign::Center),
            sum: Format::new()
                .set_bold()
                .set_background_color(sum_fill)
                .set_border(FormatBorder::Thin)
                .set_align(FormatAlign::Center),
            wrap: Format::new()
                .set_border(FormatBorder::Thin)
                .set_text_wrap()
                .set_align(FormatAlign::Top),
            note: Format::new()
                .set_border(FormatBorder::Thin)
                .set_text_wrap()
                .set_align(FormatAlign::Top),
        }
    }
}

// --- Blatt „Bewertung“ -----------------------------------------------------

/// Wo auf dem Blatt „Bewertung“ was steht (nullbasiert).
struct GradingRefs {
    /// Je Gruppe: Zelle des Gruppennamens.
    group_names: Vec<(u32, u16)>,
    /// Je Gruppe, je Punkt: (Zeile des Punktes) — Name in Spalte B, Max in C.
    item_rows: Vec<Vec<u32>>,
    /// Zelle der Gesamtpunktzahl.
    total: (u32, u16),
    /// Zeilen der Matrix, aufsteigend nach „ab“ (Spalte A Note, B ab, C bis).
    matrix_first: u32,
    matrix_last: u32,
}

impl GradingRefs {
    fn name_range(&self) -> String {
        format!(
            "{SHEET_GRADING}!{}:{}",
            row_col_to_cell_absolute(self.matrix_first, 0),
            row_col_to_cell_absolute(self.matrix_last, 0)
        )
    }

    fn start_range(&self) -> String {
        format!(
            "{SHEET_GRADING}!{}:{}",
            row_col_to_cell_absolute(self.matrix_first, 1),
            row_col_to_cell_absolute(self.matrix_last, 1)
        )
    }

    /// Die Formel, die aus Gesamtpunkten in `total_cell` die Note macht.
    fn grade_formula(&self, total_cell: &str) -> String {
        format!(
            "INDEX({},MATCH({total_cell},{},1))",
            self.name_range(),
            self.start_range()
        )
    }
}

fn grading_ref(row: u32, col: u16) -> String {
    format!("{SHEET_GRADING}!{}", row_col_to_cell_absolute(row, col))
}

fn write_grading(
    ws: &mut Worksheet,
    exam: &Exam,
    classes: &[(class::Class, Vec<StudentResult>)],
    st: &Styles,
) -> Result<GradingRefs> {
    ws.set_column_width(0, 28)?;
    ws.set_column_width(1, 32)?;
    ws.set_column_width(2, 16)?;
    ws.set_column_width(3, 44)?;

    ws.write_string_with_format(0, 0, &exam.name, &st.title)?;
    let labels: Vec<String> = classes.iter().map(|(c, _)| c.label()).collect();
    ws.write_string(1, 0, format!("Klassen: {}", labels.join(", ")))?;

    // Prüfungsschema
    let mut row = 3;
    ws.write_string_with_format(row, 0, "Prüfungsschema", &st.title)?;
    row += 1;
    for (col, text) in ["Gruppe", "Punkt", "Max. Punkte", "Beschreibung"]
        .iter()
        .enumerate()
    {
        ws.write_string_with_format(row, col as u16, *text, &st.head)?;
    }
    row += 1;

    let mut group_names = Vec::new();
    let mut item_rows = Vec::new();
    let mut sum_cells = Vec::new();
    for group in &exam.groups {
        group_names.push((row, 0u16));
        let first = row;
        let mut rows = Vec::new();
        for (i, item) in group.items.iter().enumerate() {
            if i == 0 {
                ws.write_string_with_format(row, 0, &group.name, &st.label)?;
            } else {
                ws.write_blank(row, 0, &st.label)?;
            }
            ws.write_string_with_format(row, 1, &item.name, &st.label)?;
            ws.write_number_with_format(row, 2, item.max_points, &st.input)?;
            if item.description.is_empty() {
                ws.write_blank(row, 3, &st.wrap)?;
            } else {
                ws.write_string_with_format(row, 3, &item.description, &st.wrap)?;
            }
            rows.push(row);
            row += 1;
        }
        let last = row - 1;
        ws.write_blank(row, 0, &st.sum)?;
        ws.write_formula_with_format(
            row,
            1,
            Formula::new(format!(
                r#"="Summe "&{}"#,
                row_col_to_cell_absolute(first, 0)
            )),
            &st.sum,
        )?;
        ws.write_formula_with_format(
            row,
            2,
            Formula::new(format!(
                "SUM({}:{})",
                row_col_to_cell(first, 2),
                row_col_to_cell(last, 2)
            )),
            &st.sum,
        )?;
        ws.write_blank(row, 3, &st.sum)?;
        sum_cells.push(row_col_to_cell(row, 2));
        item_rows.push(rows);
        row += 1;
    }
    ws.write_string_with_format(row, 0, "", &st.sum)?;
    ws.write_string_with_format(row, 1, "Gesamtpunkte", &st.sum)?;
    ws.write_formula_with_format(
        row,
        2,
        Formula::new(format!("={}", sum_cells.join("+"))),
        &st.sum,
    )?;
    ws.write_blank(row, 3, &st.sum)?;
    let total = (row, 2u16);
    row += 2;

    // Bewertungsmatrix
    ws.write_string_with_format(row, 0, "Bewertungsmatrix", &st.title)?;
    row += 1;
    for (col, text) in ["Note", "ab (Punkte)", "bis (Punkte)"].iter().enumerate() {
        ws.write_string_with_format(row, col as u16, *text, &st.head)?;
    }
    row += 1;
    let matrix_first = row;
    let last_index = exam.grades.len() - 1;
    for (i, grade) in exam.grades.iter().enumerate() {
        match grade.name.parse::<f64>() {
            Ok(n) => ws.write_number_with_format(row, 0, n, &st.label)?,
            Err(_) => ws.write_string_with_format(row, 0, &grade.name, &st.label)?,
        };
        ws.write_number_with_format(row, 1, grade.from_points, &st.input)?;
        let to = if i == last_index {
            format!("={}", row_col_to_cell(total.0, total.1))
        } else {
            format!("={}-0.5", row_col_to_cell(row + 1, 1))
        };
        ws.write_formula_with_format(row, 2, Formula::new(to), &st.calc)?;
        row += 1;
    }
    let matrix_last = row - 1;
    ws.write_string(
        row + 1,
        0,
        "Nur „ab“ ist eine Eingabe, „bis“ wird berechnet. Die niedrigste Note beginnt bei 0.",
    )?;

    Ok(GradingRefs {
        group_names,
        item_rows,
        total,
        matrix_first,
        matrix_last,
    })
}

// --- Klassenblatt als Matrix (ohne Begründungen) ---------------------------

const CLASS_HEAD_ROWS: u32 = 3;

fn write_class(
    ws: &mut Worksheet,
    exam: &Exam,
    g: &GradingRefs,
    results: &[StudentResult],
    st: &Styles,
) -> Result<()> {
    let student_count = results.len();
    ws.set_column_width(0, 30)?;
    ws.write_string_with_format(1, 0, "Schüler", &st.head)?;
    ws.write_string_with_format(2, 0, "Max. Punkte", &st.sum)?;
    ws.write_blank(0, 0, &st.head)?;
    for (i, res) in results.iter().enumerate() {
        ws.write_string(CLASS_HEAD_ROWS + i as u32, 0, res.student.label())?;
    }

    let first = CLASS_HEAD_ROWS;
    let mut col: u16 = 1;
    let mut item_cols: Vec<Vec<u16>> = Vec::new();
    let mut sum_cols = Vec::new();
    for (gi, group) in exam.groups.iter().enumerate() {
        let start = col;
        let mut cols = Vec::new();
        for ii in 0..group.items.len() {
            let item_row = g.item_rows[gi][ii];
            ws.set_column_width(col, 11)?;
            ws.write_formula_with_format(
                1,
                col,
                Formula::new(format!("={}", grading_ref(item_row, 1))),
                &st.head,
            )?;
            ws.write_formula_with_format(
                2,
                col,
                Formula::new(format!("={}", grading_ref(item_row, 2))),
                &st.sum,
            )?;
            for (r, res) in results.iter().enumerate() {
                let row = first + r as u32;
                match res.groups[gi].scores[ii].points {
                    Some(p) => ws.write_number_with_format(row, col, p, &st.input)?,
                    None => ws.write_blank(row, col, &st.input)?,
                };
            }
            cols.push(col);
            col += 1;
        }
        // Summe der Gruppe
        let (gr, gc) = g.group_names[gi];
        ws.set_column_width(col, 12)?;
        ws.write_formula_with_format(
            1,
            col,
            Formula::new(format!(r#"="Summe "&{}"#, grading_ref(gr, gc))),
            &st.head,
        )?;
        ws.write_formula_with_format(
            2,
            col,
            Formula::new(format!(
                "SUM({}:{})",
                row_col_to_cell(2, start),
                row_col_to_cell(2, col - 1)
            )),
            &st.sum,
        )?;
        for r in 0..student_count as u32 {
            let row = first + r;
            ws.write_formula_with_format(
                row,
                col,
                Formula::new(format!(
                    "SUM({}:{})",
                    row_col_to_cell(row, start),
                    row_col_to_cell(row, col - 1)
                )),
                &st.sum,
            )?;
        }
        // Gruppenname über der Gruppe (verbunden, per Formel).
        if col > start {
            ws.merge_range(0, start, 0, col, "", &st.head)?;
        }
        ws.write_formula_with_format(
            0,
            start,
            Formula::new(format!("={}", grading_ref(gr, gc))),
            &st.head,
        )?;
        sum_cols.push(col);
        item_cols.push(cols);
        col += 1;
    }

    let total_col = col;
    let grade_col = col + 1;
    ws.set_column_width(total_col, 12)?;
    ws.set_column_width(grade_col, 10)?;
    ws.write_blank(0, total_col, &st.head)?;
    ws.write_blank(0, grade_col, &st.head)?;
    ws.write_string_with_format(1, total_col, "Gesamt", &st.head)?;
    ws.write_string_with_format(1, grade_col, "Note", &st.head)?;
    ws.write_formula_with_format(
        2,
        total_col,
        Formula::new(format!("={}", grading_ref(g.total.0, g.total.1))),
        &st.sum,
    )?;
    ws.write_blank(2, grade_col, &st.sum)?;

    for r in 0..student_count as u32 {
        let row = first + r;
        let sums: Vec<String> = sum_cols.iter().map(|c| row_col_to_cell(row, *c)).collect();
        ws.write_formula_with_format(
            row,
            total_col,
            Formula::new(format!("={}", sums.join("+"))),
            &st.sum,
        )?;
        // Solange nichts eingetragen ist, bleibt die Note leer.
        let ranges: Vec<String> = item_cols
            .iter()
            .filter(|c| !c.is_empty())
            .map(|c| {
                format!(
                    "{}:{}",
                    row_col_to_cell(row, c[0]),
                    row_col_to_cell(row, c[c.len() - 1])
                )
            })
            .collect();
        let total_cell = row_col_to_cell(row, total_col);
        ws.write_formula_with_format(
            row,
            grade_col,
            Formula::new(format!(
                r#"=IF(COUNT({})=0,"",{})"#,
                ranges.join(","),
                g.grade_formula(&total_cell)
            )),
            &st.sum,
        )?;
    }

    ws.set_freeze_panes(first, 1)?;
    ws.set_landscape();
    ws.set_paper_size(9);
    ws.set_print_fit_to_pages(1, 0);
    ws.set_repeat_rows(0, 2)?;
    Ok(())
}

// --- Klassenblatt als Bewertungskacheln (mit Begründungen) -----------------

/// Spaltenbreiten der Kachel: Aufgabe, Beschreibung, Punkte, Max., Begründung.
const TILE_WIDTHS: [f64; 5] = [26.0, 32.0, 9.0, 9.0, 40.0];
/// Druckmaßstab in Prozent: Die Kachel (rund 116 Zeichen breit) passt damit
/// auf A4 hochkant. Ein fester Maßstab statt „auf eine Seite anpassen“, weil
/// Excel bei Letzterem die Seitenumbrüche zwischen den Kacheln ignoriert.
const TILE_PRINT_SCALE: u16 = 85;

/// Zeilen, die `text` in einer Spalte von `width` Zeichen belegt (grob).
fn lines_needed(text: &str, width: f64) -> usize {
    let per_line = (width * 0.9).max(1.0) as usize;
    text.lines()
        .map(|l| l.chars().count().max(1).div_ceil(per_line))
        .sum::<usize>()
        .max(1)
}

fn write_tiles(
    ws: &mut Worksheet,
    exam: &Exam,
    g: &GradingRefs,
    class_label: &str,
    results: &[StudentResult],
    st: &Styles,
) -> Result<()> {
    for (col, width) in TILE_WIDTHS.iter().enumerate() {
        ws.set_column_width(col as u16, *width)?;
    }
    ws.set_portrait();
    ws.set_paper_size(9);
    ws.set_print_scale(TILE_PRINT_SCALE);
    ws.set_margins(0.4, 0.4, 0.5, 0.5, 0.3, 0.3);

    if results.is_empty() {
        ws.write_string(0, 0, "Keine Schüler in dieser Klasse.")?;
        return Ok(());
    }

    // Jede Kachel beginnt auf einer neuen Seite.
    let mut breaks = Vec::new();
    let mut top: u32 = 0;
    for res in results {
        if top > 0 {
            breaks.push(top);
        }
        top = write_tile(ws, exam, g, class_label, res, top, st)?;
    }
    ws.set_page_breaks(&breaks)?;
    Ok(())
}

/// Schreibt die Kachel eines Schülers ab Zeile `top`; gibt die Zeile zurück,
/// ab der die nächste beginnt.
fn write_tile(
    ws: &mut Worksheet,
    exam: &Exam,
    g: &GradingRefs,
    class_label: &str,
    res: &StudentResult,
    top: u32,
    st: &Styles,
) -> Result<u32> {
    let mut r = top;
    ws.write_string_with_format(r, 0, &exam.name, &st.title)?;
    r += 1;
    ws.write_string_with_format(r, 0, "Schüler/in", &st.head)?;
    ws.merge_range(r, 1, r, 4, &res.student.label(), &st.label)?;
    r += 1;
    ws.write_string_with_format(r, 0, "Klasse", &st.head)?;
    ws.merge_range(r, 1, r, 4, class_label, &st.label)?;
    r += 2;

    for (col, text) in ["Aufgabe", "Beschreibung", "Punkte", "Max.", "Begründung"]
        .iter()
        .enumerate()
    {
        ws.write_string_with_format(r, col as u16, *text, &st.head)?;
    }
    r += 1;

    let mut group_rows = Vec::new();
    let mut ranges = Vec::new();
    for (gi, group) in exam.groups.iter().enumerate() {
        let (gr, gc) = g.group_names[gi];
        let head_row = r;
        let first_item = r + 1;
        let last_item = r + group.items.len() as u32;
        ws.write_formula_with_format(
            head_row,
            0,
            Formula::new(format!("={}", grading_ref(gr, gc))),
            &st.sum,
        )?;
        ws.write_blank(head_row, 1, &st.sum)?;
        for col in [2u16, 3] {
            ws.write_formula_with_format(
                head_row,
                col,
                Formula::new(format!(
                    "SUM({}:{})",
                    row_col_to_cell(first_item, col),
                    row_col_to_cell(last_item, col)
                )),
                &st.sum,
            )?;
        }
        ws.write_blank(head_row, 4, &st.sum)?;
        group_rows.push(head_row);
        ranges.push(format!(
            "{}:{}",
            row_col_to_cell(first_item, 2),
            row_col_to_cell(last_item, 2)
        ));
        r += 1;

        for (ii, item) in group.items.iter().enumerate() {
            let item_row = g.item_rows[gi][ii];
            let score = &res.groups[gi].scores[ii];
            ws.write_formula_with_format(
                r,
                0,
                Formula::new(format!("={}", grading_ref(item_row, 1))),
                &st.label,
            )?;
            // Beschreibung und Begründung als Text, nicht als Formel: Excel
            // passt die Zeilenhöhe an Formelergebnisse erst nach dem Rechnen an.
            if item.description.is_empty() {
                ws.write_blank(r, 1, &st.wrap)?;
            } else {
                ws.write_string_with_format(r, 1, &item.description, &st.wrap)?;
            }
            match score.points {
                Some(p) => ws.write_number_with_format(r, 2, p, &st.input)?,
                None => ws.write_blank(r, 2, &st.input)?,
            };
            ws.write_formula_with_format(
                r,
                3,
                Formula::new(format!("={}", grading_ref(item_row, 2))),
                &st.calc,
            )?;
            if score.note.is_empty() {
                ws.write_blank(r, 4, &st.note)?;
            } else {
                ws.write_string_with_format(r, 4, &score.note, &st.note)?;
            }
            let lines = lines_needed(&item.description, TILE_WIDTHS[1])
                .max(lines_needed(&score.note, TILE_WIDTHS[4]));
            if lines > 1 {
                ws.set_row_height(r, 15.0 * lines as f64)?;
            }
            r += 1;
        }
    }

    // Gesamtpunkte
    let sums = |col: u16| -> String {
        group_rows
            .iter()
            .map(|row| row_col_to_cell(*row, col))
            .collect::<Vec<_>>()
            .join("+")
    };
    ws.write_string_with_format(r, 0, "Gesamtpunkte", &st.sum)?;
    ws.write_blank(r, 1, &st.sum)?;
    ws.write_formula_with_format(r, 2, Formula::new(format!("={}", sums(2))), &st.sum)?;
    ws.write_formula_with_format(
        r,
        3,
        Formula::new(format!("={}", grading_ref(g.total.0, g.total.1))),
        &st.sum,
    )?;
    ws.write_blank(r, 4, &st.sum)?;
    let total_cell = row_col_to_cell(r, 2);
    r += 1;

    // Note: Solange nichts eingetragen ist, bleibt sie leer.
    ws.write_string_with_format(r, 0, "Note", &st.sum)?;
    ws.write_blank(r, 1, &st.sum)?;
    ws.write_formula_with_format(
        r,
        2,
        Formula::new(format!(
            r#"=IF(COUNT({})=0,"",{})"#,
            ranges.join(","),
            g.grade_formula(&total_cell)
        )),
        &st.sum,
    )?;
    ws.write_blank(r, 3, &st.sum)?;
    ws.write_blank(r, 4, &st.sum)?;
    r += 2;

    // Bewertungsmatrix zur Einordnung.
    ws.write_string_with_format(r, 0, "Bewertungsmatrix", &st.title)?;
    r += 1;
    ws.write_string_with_format(r, 0, "Note", &st.head)?;
    ws.write_string_with_format(r, 1, "Punkte (ab – bis)", &st.head)?;
    r += 1;
    for src in g.matrix_first..=g.matrix_last {
        ws.write_formula_with_format(
            r,
            0,
            Formula::new(format!("={}", grading_ref(src, 0))),
            &st.label,
        )?;
        ws.write_formula_with_format(
            r,
            1,
            Formula::new(format!(
                r#"={}&" – "&{}"#,
                grading_ref(src, 1),
                grading_ref(src, 2)
            )),
            &st.calc,
        )?;
        r += 1;
    }
    Ok(r)
}

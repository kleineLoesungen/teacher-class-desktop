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
//! * **je Klasse ein Blatt** (Blattname `7a (2025)`) als **Matrix** — Schüler
//!   in den Zeilen, Punkte des Schemas in den Spalten, Summe je Gruppe,
//!   Gesamtpunkte, Note. Begründungen stehen nicht in der Excel-Datei; zum
//!   Ausdrucken (eine Seite je Schüler, mit Begründungen) gibt es den
//!   PDF-Export in [`crate::export_pdf`].
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
        write_class(&mut ws, &exam, &grading_refs, results, &styles)?;
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

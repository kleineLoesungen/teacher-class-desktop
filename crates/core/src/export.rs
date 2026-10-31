//! Excel-Export einer Prüfung.
//!
//! Die Datei ist eine Vorlage zum Ausfüllen: Die Lehrkraft trägt in Excel die
//! Punkte je Schüler ein, alles Weitere rechnen **Formeln**. Ändert sie die
//! Bewertungsmatrix (oder eine Höchstpunktzahl) auf dem Blatt „Bewertung“,
//! ändern sich die Noten.
//!
//! Blätter:
//!
//! * **Bewertung** — Prüfungsschema (Gruppen, Punkte, Höchstpunktzahlen,
//!   Summen) und Bewertungsmatrix (Note, ab, bis). „ab“ und die
//!   Höchstpunktzahlen sind Eingaben, alles andere Formeln.
//! * **Ausdruck** — eine druckfertige Seite für einen Schüler, der oben aus
//!   einer Liste gewählt wird: Punkte je Aufgabe, Summen, Note und die
//!   Bewertungsmatrix. Gedruckt oder als PDF gespeichert wird in Excel.
//! * **je Klasse ein Blatt** (Blattname `7a (2025)`) — Schüler in den Zeilen,
//!   Punkte des Schemas in den Spalten, Summe je Gruppe, Gesamtpunkte, Note.
//! * **Listen** — verborgene Hilfsliste für die Auswahl auf „Ausdruck“.
//!
//! Die Formeln sind in `crates/core/tests/export.rs` nur als Text geprüft;
//! gerechnet hat sie dort niemand. Ein Blick in Excel oder LibreOffice
//! bleibt nötig.

use std::path::Path;

use rust_xlsxwriter::utility::{row_col_to_cell, row_col_to_cell_absolute};
use rust_xlsxwriter::{
    Color, DataValidation, Format, FormatAlign, FormatBorder, Formula, Workbook, Worksheet,
    XlsxError,
};
use uuid::Uuid;

use crate::error::{Error, Result, Validator};
use crate::exam::{self, Exam};
use crate::{class, student};

/// Blattname der Bewertung.
pub const SHEET_GRADING: &str = "Bewertung";
/// Blattname des Ausdrucks.
pub const SHEET_PRINT: &str = "Ausdruck";
const SHEET_LISTS: &str = "Listen";

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
        let students = student::list(conn, *id)?;
        classes.push((cls, students));
    }

    let styles = Styles::new();
    let mut grading = Worksheet::new();
    grading.set_name(SHEET_GRADING)?;
    let grading_refs = write_grading(&mut grading, &exam, &classes, &styles)?;

    // Klassenblätter. Alle haben dieselben Spalten, nur die Zeilen
    // unterscheiden sich.
    let mut class_sheets = Vec::new();
    let mut lists: Vec<(String, String, u32)> = Vec::new();
    let mut layout = None;
    for (cls, students) in &classes {
        let mut ws = Worksheet::new();
        ws.set_name(cls.label())?;
        let l = write_class(&mut ws, &exam, &grading_refs, students.len(), &styles)?;
        let mut counts = std::collections::HashMap::<String, u32>::new();
        for (i, s) in students.iter().enumerate() {
            let row = l.first_student_row + i as u32;
            ws.write_string(row, 0, s.label())?;
            // Für die Auswahl auf „Ausdruck“: eindeutig auch bei gleichen Namen.
            let base = format!("{}: {}", cls.label(), s.label());
            let n = counts.entry(base.clone()).or_insert(0);
            *n += 1;
            let label = if *n == 1 {
                base
            } else {
                format!("{base} ({n})")
            };
            lists.push((label, cls.label(), row + 1));
        }
        layout = Some(l);
        class_sheets.push(ws);
    }

    let mut workbook = Workbook::new();
    workbook.push_worksheet(grading);

    // Die Ausdrucksseite braucht mindestens einen Schüler zum Auswählen.
    let mut lists_sheet = None;
    if let (Some(layout), false) = (&layout, lists.is_empty()) {
        let mut print = Worksheet::new();
        print.set_name(SHEET_PRINT)?;
        write_print(&mut print, &exam, &grading_refs, layout, &lists, &styles)?;
        workbook.push_worksheet(print);

        let mut sheet = Worksheet::new();
        sheet.set_name(SHEET_LISTS)?;
        for (i, (label, class_sheet, row)) in lists.iter().enumerate() {
            let r = i as u32;
            sheet.write_string(r, 0, label)?;
            sheet.write_string(r, 1, class_sheet)?;
            sheet.write_number(r, 2, f64::from(*row))?;
        }
        sheet.set_hidden(true);
        lists_sheet = Some(sheet);
    }
    for ws in class_sheets {
        workbook.push_worksheet(ws);
    }
    if let Some(sheet) = lists_sheet {
        workbook.push_worksheet(sheet);
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
    classes: &[(class::Class, Vec<student::Student>)],
    st: &Styles,
) -> Result<GradingRefs> {
    ws.set_column_width(0, 28)?;
    ws.set_column_width(1, 32)?;
    ws.set_column_width(2, 16)?;

    ws.write_string_with_format(0, 0, &exam.name, &st.title)?;
    let labels: Vec<String> = classes.iter().map(|(c, _)| c.label()).collect();
    ws.write_string(1, 0, format!("Klassen: {}", labels.join(", ")))?;

    // Prüfungsschema
    let mut row = 3;
    ws.write_string_with_format(row, 0, "Prüfungsschema", &st.title)?;
    row += 1;
    for (col, text) in ["Gruppe", "Punkt", "Max. Punkte"].iter().enumerate() {
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

// --- Klassenblatt ----------------------------------------------------------

/// Spalten und Zeilen eines Klassenblatts (nullbasiert).
struct ClassLayout {
    first_student_row: u32,
    /// Spalte der Punkte, je Gruppe.
    item_cols: Vec<Vec<u16>>,
    /// Spalte der Gruppensumme, je Gruppe.
    sum_cols: Vec<u16>,
    total_col: u16,
    grade_col: u16,
}

const CLASS_HEAD_ROWS: u32 = 3;

fn write_class(
    ws: &mut Worksheet,
    exam: &Exam,
    g: &GradingRefs,
    student_count: usize,
    st: &Styles,
) -> Result<ClassLayout> {
    ws.set_column_width(0, 30)?;
    ws.write_string_with_format(1, 0, "Schüler", &st.head)?;
    ws.write_string_with_format(2, 0, "Max. Punkte", &st.sum)?;
    ws.write_blank(0, 0, &st.head)?;

    let first = CLASS_HEAD_ROWS;
    let mut col: u16 = 1;
    let mut item_cols = Vec::new();
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
            for r in 0..student_count as u32 {
                ws.write_blank(first + r, col, &st.input)?;
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
    Ok(ClassLayout {
        first_student_row: first,
        item_cols,
        sum_cols,
        total_col,
        grade_col,
    })
}

// --- Blatt „Ausdruck“ ------------------------------------------------------

fn write_print(
    ws: &mut Worksheet,
    exam: &Exam,
    g: &GradingRefs,
    layout: &ClassLayout,
    lists: &[(String, String, u32)],
    st: &Styles,
) -> Result<()> {
    let list_len = lists.len();
    ws.set_column_width(0, 34)?;
    ws.set_column_width(1, 14)?;
    ws.set_column_width(2, 14)?;
    // Hilfszellen rechts neben dem Druckbereich.
    ws.set_column_width(5, 22)?;
    ws.set_column_width(6, 12)?;

    let list_a = format!("Listen!$A$1:$A${list_len}");
    let list_b = format!("Listen!$B$1:$B${list_len}");
    let list_c = format!("Listen!$C$1:$C${list_len}");

    ws.write_string_with_format(0, 0, "Schüler/in (Auswahl)", &st.head)?;
    ws.merge_range(0, 1, 0, 2, "", &st.label)?;
    ws.write_string_with_format(0, 1, &lists[0].0, &st.label)?;
    ws.add_data_validation(
        0,
        1,
        0,
        1,
        &DataValidation::new().allow_list_formula(Formula::new(list_a.clone())),
    )?;

    // Hilfszellen: Blatt und Zeile des gewählten Schülers.
    ws.write_string(0, 5, "Hilfszelle: Klassenblatt")?;
    ws.write_formula(
        1,
        5,
        Formula::new(format!("=INDEX({list_b},MATCH($B$1,{list_a},0))")),
    )?;
    ws.write_string(0, 6, "Zeile")?;
    ws.write_formula(
        1,
        6,
        Formula::new(format!("=INDEX({list_c},MATCH($B$1,{list_a},0))")),
    )?;
    let sheet = "$F$2";
    let row = "$G$2";
    // Zelle des gewählten Schülers im Klassenblatt, Spalte `col` (nullbasiert).
    let pick = |col: u16| format!(r#"INDIRECT("'"&{sheet}&"'!"&ADDRESS({row},{}))"#, col + 1);

    ws.write_string_with_format(1, 0, "Prüfung", &st.head)?;
    ws.merge_range(1, 1, 1, 2, &exam.name, &st.label)?;
    ws.write_string_with_format(2, 0, "Klasse", &st.head)?;
    ws.merge_range(2, 1, 2, 2, "", &st.label)?;
    ws.write_formula_with_format(2, 1, Formula::new(format!("={sheet}")), &st.label)?;

    let mut r: u32 = 4;
    for (col, text) in ["Aufgabe", "Punkte", "Max. Punkte"].iter().enumerate() {
        ws.write_string_with_format(r, col as u16, *text, &st.head)?;
    }
    r += 1;
    for (gi, group) in exam.groups.iter().enumerate() {
        let (gr, gc) = g.group_names[gi];
        ws.write_formula_with_format(
            r,
            0,
            Formula::new(format!("={}", grading_ref(gr, gc))),
            &st.sum,
        )?;
        ws.write_blank(r, 1, &st.sum)?;
        ws.write_blank(r, 2, &st.sum)?;
        r += 1;
        for ii in 0..group.items.len() {
            let item_row = g.item_rows[gi][ii];
            let value = pick(layout.item_cols[gi][ii]);
            ws.write_formula_with_format(
                r,
                0,
                Formula::new(format!("={}", grading_ref(item_row, 1))),
                &st.label,
            )?;
            ws.write_formula_with_format(
                r,
                1,
                Formula::new(format!(r#"=IF({value}="","",{value})"#)),
                &st.input,
            )?;
            ws.write_formula_with_format(
                r,
                2,
                Formula::new(format!("={}", grading_ref(item_row, 2))),
                &st.calc,
            )?;
            r += 1;
        }
        ws.write_string_with_format(r, 0, "Summe", &st.sum)?;
        ws.write_formula_with_format(
            r,
            1,
            Formula::new(format!("={}", pick(layout.sum_cols[gi]))),
            &st.sum,
        )?;
        // Höchstpunktzahl der Gruppe: Summe der Punkte oben.
        let first_item = r - group.items.len() as u32;
        ws.write_formula_with_format(
            r,
            2,
            Formula::new(format!(
                "=SUM({}:{})",
                row_col_to_cell(first_item, 2),
                row_col_to_cell(r - 1, 2)
            )),
            &st.sum,
        )?;
        r += 1;
    }
    ws.write_string_with_format(r, 0, "Gesamtpunkte", &st.sum)?;
    ws.write_formula_with_format(
        r,
        1,
        Formula::new(format!("={}", pick(layout.total_col))),
        &st.sum,
    )?;
    ws.write_formula_with_format(
        r,
        2,
        Formula::new(format!("={}", grading_ref(g.total.0, g.total.1))),
        &st.sum,
    )?;
    r += 1;
    ws.write_string_with_format(r, 0, "Note", &st.sum)?;
    ws.write_formula_with_format(
        r,
        1,
        Formula::new(format!("={}", pick(layout.grade_col))),
        &st.sum,
    )?;
    ws.write_blank(r, 2, &st.sum)?;
    r += 2;

    // Bewertungsmatrix zur Einschätzung.
    ws.write_string_with_format(r, 0, "Bewertungsmatrix", &st.title)?;
    r += 1;
    for (col, text) in ["Note", "ab (Punkte)", "bis (Punkte)"].iter().enumerate() {
        ws.write_string_with_format(r, col as u16, *text, &st.head)?;
    }
    r += 1;
    for k in 0..=(g.matrix_last - g.matrix_first) {
        let src = g.matrix_first + k;
        for col in 0..3u16 {
            ws.write_formula_with_format(
                r,
                col,
                Formula::new(format!("={}", grading_ref(src, col))),
                if col == 0 { &st.label } else { &st.calc },
            )?;
        }
        r += 1;
    }

    ws.set_print_area(0, 0, r - 1, 2)?;
    ws.set_paper_size(9);
    ws.set_print_fit_to_pages(1, 1);
    Ok(())
}

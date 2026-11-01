//! PDF-Ausdruck: eine Seite je Schüler, ein PDF je Klasse.
//!
//! Gesehen hat die Seiten hier niemand; der Test prüft Aufbau, Seitenzahl und
//! dass die Texte im Inhaltsstrom stehen (Hex-Zeichenketten in WinAnsi).

use teacher_class_core::class::{self, ClassInput};
use teacher_class_core::exam::{self, ExamInput, GradeInput, GroupInput, ItemInput};
use teacher_class_core::grading::{self, ScoreInput};
use teacher_class_core::{Error, db, export_pdf, student};

fn beispiel(class_ids: Vec<uuid::Uuid>) -> ExamInput {
    let item = |name: &str, max: f64| ItemInput {
        name: name.into(),
        max_points: max,
        ..Default::default()
    };
    ExamInput {
        name: "Klassenarbeit".into(),
        class_ids,
        groups: vec![
            GroupInput {
                name: "Aufgabe 1".into(),
                items: vec![item("a", 4.0), item("b", 2.5)],
            },
            GroupInput {
                name: "Aufgabe 2".into(),
                items: vec![item("a", 13.5)],
            },
        ],
        grades: vec![
            GradeInput {
                name: "1".into(),
                from_points: 17.0,
            },
            GradeInput {
                name: "6".into(),
                from_points: 0.0,
            },
        ],
        with_notes: true,
    }
}

/// Wie das PDF einen Text schreibt: WinAnsi-Bytes als Hex.
fn hex(text: &str) -> String {
    text.chars()
        .map(|c| {
            let byte = match c {
                'ä' => 0xE4,
                'ü' => 0xFC,
                'ö' => 0xF6,
                c => c as u8,
            };
            format!("{byte:02X}")
        })
        .collect()
}

fn text(bytes: &[u8]) -> String {
    String::from_utf8(bytes.to_vec()).expect("das PDF ist reines ASCII")
}

fn pages(pdf: &str) -> usize {
    pdf.matches("/Type /Page /").count()
}

#[test]
fn ein_pdf_je_klasse_mit_einer_seite_je_schueler() {
    let conn = db::open_in_memory().unwrap();
    let a = class::create(&conn, ClassInput::new("7a", 2025)).unwrap();
    let b = class::create(&conn, ClassInput::new("7b", 2025)).unwrap();
    let s = student::add_many(&conn, a.id, "Muster, Max\nBeispiel, Eva\nÄhnlich, Uwe").unwrap();
    student::add_many(&conn, b.id, "Test, Tom").unwrap();
    let mut input = beispiel(vec![a.id, b.id]);
    input.groups[0].items[0].description = "Rechenweg angeben".into();
    let e = exam::create(&conn, input).unwrap();
    grading::set(
        &conn,
        e.id,
        ScoreInput {
            student_id: s[0].id,
            group: 0,
            item: 0,
            points: Some(3.0),
            note: "Vorzeichenfehler".into(),
        },
    )
    .unwrap();

    let pdf = text(&export_pdf::build(&conn, e.id, a.id).unwrap());
    assert!(pdf.starts_with("%PDF-1.4"));
    assert!(pdf.trim_end().ends_with("%%EOF"));
    assert_eq!(pages(&pdf), 3, "eine Seite je Schüler der Klasse 7a");
    assert!(pdf.contains("/BaseFont /Times-Roman"));
    // Schüler, Klasse, Begründung und Beschreibung stehen drin.
    for expected in [
        "Muster, Max",
        "Beispiel, Eva",
        "Ähnlich, Uwe",
        "7a (2025)",
        "Vorzeichenfehler",
        "Rechenweg angeben",
        "Bewertungsmatrix",
    ] {
        let wanted = if expected.contains('Ä') {
            // Ä ist 0xC4 in WinAnsi.
            hex(&expected.replace('Ä', "A")).replacen("41", "C4", 1)
        } else {
            hex(expected)
        };
        assert!(pdf.contains(&wanted), "{expected} fehlt");
    }
    // Die andere Klasse ist nicht dabei.
    assert!(!pdf.contains(&hex("Test, Tom")));

    let other = text(&export_pdf::build(&conn, e.id, b.id).unwrap());
    assert_eq!(pages(&other), 1);
    assert!(other.contains(&hex("Test, Tom")));
}

#[test]
fn die_begruendung_steht_nur_bei_pruefungen_mit_begruendungen() {
    let conn = db::open_in_memory().unwrap();
    let a = class::create(&conn, ClassInput::new("7a", 2025)).unwrap();
    student::add_many(&conn, a.id, "Muster, Max").unwrap();
    let mut input = beispiel(vec![a.id]);
    input.with_notes = false;
    let e = exam::create(&conn, input).unwrap();

    let pdf = text(&export_pdf::build(&conn, e.id, a.id).unwrap());
    assert!(!pdf.contains(&hex("Begründung")));
    assert_eq!(pages(&pdf), 1);
}

#[test]
fn eine_sehr_lange_begruendung_laeuft_auf_die_naechste_seite() {
    let conn = db::open_in_memory().unwrap();
    let a = class::create(&conn, ClassInput::new("7a", 2025)).unwrap();
    let s = student::add_many(&conn, a.id, "Muster, Max\nBeispiel, Eva").unwrap();
    let e = exam::create(&conn, beispiel(vec![a.id])).unwrap();
    // Höchstens 2000 Zeichen je Begründung; drei davon sprengen eine Seite.
    for (group, item) in [(0, 0), (0, 1), (1, 0)] {
        grading::set(
            &conn,
            e.id,
            ScoreInput {
                student_id: s[0].id,
                group,
                item,
                points: Some(1.0),
                note: "Rechenweg fehlt vollständig und die Einheit ist falsch. ".repeat(33),
            },
        )
        .unwrap();
    }

    let pdf = text(&export_pdf::build(&conn, e.id, a.id).unwrap());
    // Max Muster braucht mehr als eine Seite, Eva Beispiel beginnt trotzdem neu.
    assert!(pages(&pdf) >= 3, "{} Seiten", pages(&pdf));
    // Der Tabellenkopf wiederholt sich auf der Folgeseite.
    assert!(pdf.matches(&hex("Aufgabe")).count() >= 4);
}

#[test]
fn eine_klasse_ohne_schueler_ergibt_eine_seite() {
    let conn = db::open_in_memory().unwrap();
    let a = class::create(&conn, ClassInput::new("7a", 2025)).unwrap();
    let e = exam::create(&conn, beispiel(vec![a.id])).unwrap();

    let pdf = text(&export_pdf::build(&conn, e.id, a.id).unwrap());
    assert_eq!(pages(&pdf), 1);
    assert!(pdf.contains(&hex("Keine Schüler in dieser Klasse.")));
}

#[test]
fn zeichen_ausserhalb_von_winansi_werden_zu_fragezeichen() {
    let conn = db::open_in_memory().unwrap();
    let a = class::create(&conn, ClassInput::new("7a", 2025)).unwrap();
    student::add_many(&conn, a.id, "Łukasz, Ola").unwrap();
    let e = exam::create(&conn, beispiel(vec![a.id])).unwrap();

    let pdf = text(&export_pdf::build(&conn, e.id, a.id).unwrap());
    assert!(pdf.contains(&hex("?ukasz, Ola")));
}

#[test]
fn eine_klasse_ausserhalb_der_pruefung_wird_abgelehnt() {
    let conn = db::open_in_memory().unwrap();
    let a = class::create(&conn, ClassInput::new("7a", 2025)).unwrap();
    let b = class::create(&conn, ClassInput::new("7b", 2025)).unwrap();
    let e = exam::create(&conn, beispiel(vec![a.id])).unwrap();

    let err = export_pdf::build(&conn, e.id, b.id).unwrap_err();
    assert_eq!(err.fields().unwrap()[0].field, "class");
    let err = export_pdf::build(&conn, uuid::Uuid::now_v7(), a.id).unwrap_err();
    assert!(matches!(err, Error::NotFound));
}

#[test]
fn speichern_verlangt_pdf_und_schreibt_die_datei() {
    let conn = db::open_in_memory().unwrap();
    let a = class::create(&conn, ClassInput::new("7a", 2025)).unwrap();
    let e = exam::create(&conn, beispiel(vec![a.id])).unwrap();
    let dir = tempfile::tempdir().unwrap();

    let err = export_pdf::write_pdf(&conn, e.id, a.id, &dir.path().join("a.txt")).unwrap_err();
    assert_eq!(err.fields().unwrap()[0].field, "path");

    let path = dir.path().join("Prüfung.pdf");
    export_pdf::write_pdf(&conn, e.id, a.id, &path).unwrap();
    assert_eq!(&std::fs::read(&path).unwrap()[..5], b"%PDF-");
}

//! Excel-Export: Aufbau der Datei und Formeln.
//!
//! Gerechnet werden die Formeln hier nicht — das täte erst Excel oder
//! LibreOffice. Der Test liest die erzeugte XML und prüft, dass die Blätter
//! da sind und die Formeln auf die Bewertung zeigen.

use std::io::{Cursor, Read};

use teacher_class_core::class::{self, ClassInput};
use teacher_class_core::exam::{self, ExamInput, GradeInput, GroupInput, ItemInput};
use teacher_class_core::{Error, db, export, student};

struct Xlsx(zip::ZipArchive<Cursor<Vec<u8>>>);

impl Xlsx {
    fn open(bytes: Vec<u8>) -> Self {
        Self(zip::ZipArchive::new(Cursor::new(bytes)).expect("gültiges ZIP"))
    }

    fn text(&mut self, name: &str) -> String {
        let mut s = String::new();
        self.0
            .by_name(name)
            .unwrap_or_else(|_| panic!("{name} fehlt"))
            .read_to_string(&mut s)
            .unwrap();
        s
    }
}

fn beispiel(class_ids: Vec<uuid::Uuid>) -> ExamInput {
    let item = |name: &str, max: f64| ItemInput {
        name: name.into(),
        max_points: max,
    };
    let grade = |name: &str, from: f64| GradeInput {
        name: name.into(),
        from_points: from,
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
        grades: vec![grade("1", 17.0), grade("3", 10.0), grade("6", 0.0)],
    }
}

#[test]
fn die_datei_hat_die_erwarteten_blaetter_in_reihenfolge() {
    let conn = db::open_in_memory().unwrap();
    let a = class::create(&conn, ClassInput::new("7a", 2025)).unwrap();
    let b = class::create(&conn, ClassInput::new("7b", 2024)).unwrap();
    student::add_many(&conn, a.id, "Muster, Max\nBeispiel, Eva").unwrap();
    student::add_many(&conn, b.id, "Test, Tom").unwrap();
    let e = exam::create(&conn, beispiel(vec![a.id, b.id])).unwrap();

    let mut x = Xlsx::open(export::build(&conn, e.id).unwrap());
    let wb = x.text("xl/workbook.xml");
    let order: Vec<_> = ["Bewertung", "Ausdruck", "7a (2025)", "7b (2024)", "Listen"]
        .iter()
        .map(|name| {
            wb.find(&format!("name=\"{name}\""))
                .unwrap_or_else(|| panic!("{name} fehlt"))
        })
        .collect();
    assert!(order.windows(2).all(|w| w[0] < w[1]), "Reihenfolge: {wb}");
    assert!(wb.contains("state=\"hidden\""), "Listen ist verborgen");
}

#[test]
fn das_klassenblatt_rechnet_summen_und_note_per_formel() {
    let conn = db::open_in_memory().unwrap();
    let a = class::create(&conn, ClassInput::new("7a", 2025)).unwrap();
    student::add_many(&conn, a.id, "Muster, Max").unwrap();
    let e = exam::create(&conn, beispiel(vec![a.id])).unwrap();

    let mut x = Xlsx::open(export::build(&conn, e.id).unwrap());
    // Blatt 1 Bewertung, 2 Ausdruck, 3 die Klasse.
    let sheet = x.text("xl/worksheets/sheet3.xml");
    // Summe der ersten Gruppe (B..C) in Spalte D, Zeile 4.
    assert!(sheet.contains("<f>SUM(B4:C4)</f>"), "{sheet}");
    // Gesamtpunkte: Summe der Gruppensummen D und F.
    assert!(sheet.contains("<f>D4+F4</f>"), "{sheet}");
    // Note aus der Matrix auf dem Blatt Bewertung.
    assert!(sheet.contains("INDEX(Bewertung!$A$"), "{sheet}");
    assert!(sheet.contains("MATCH(G4,Bewertung!$B$"), "{sheet}");
    // Die Höchstpunkte kommen aus dem Schema, nicht als feste Zahl.
    assert!(sheet.contains("<f>Bewertung!$C$"), "{sheet}");
}

#[test]
fn die_bewertung_hat_summen_und_bis_als_formeln() {
    let conn = db::open_in_memory().unwrap();
    let a = class::create(&conn, ClassInput::new("7a", 2025)).unwrap();
    let e = exam::create(&conn, beispiel(vec![a.id])).unwrap();

    let mut x = Xlsx::open(export::build(&conn, e.id).unwrap());
    // Ohne Schüler gibt es keinen „Ausdruck“: Bewertung, dann die Klasse.
    let sheet = x.text("xl/worksheets/sheet1.xml");
    assert!(sheet.contains("<f>SUM(C6:C7)</f>"), "{sheet}");
    assert!(
        sheet.contains("-0.5</f>"),
        "bis = nächstes ab - 0,5: {sheet}"
    );
    let wb = x.text("xl/workbook.xml");
    assert!(!wb.contains("Ausdruck"), "ohne Schüler kein Ausdruck");
}

#[test]
fn der_ausdruck_holt_die_werte_des_gewaehlten_schuelers() {
    let conn = db::open_in_memory().unwrap();
    let a = class::create(&conn, ClassInput::new("7a", 2025)).unwrap();
    student::add_many(&conn, a.id, "Muster, Max").unwrap();
    let e = exam::create(&conn, beispiel(vec![a.id])).unwrap();

    let mut x = Xlsx::open(export::build(&conn, e.id).unwrap());
    let sheet = x.text("xl/worksheets/sheet2.xml");
    assert!(sheet.contains("INDIRECT"), "{sheet}");
    assert!(
        sheet.contains("<dataValidation"),
        "Auswahl des Schülers: {sheet}"
    );
    assert!(sheet.contains("Listen!$A$1:$A$1"), "{sheet}");
}

#[test]
fn speichern_verlangt_xlsx_und_schreibt_die_datei() {
    let conn = db::open_in_memory().unwrap();
    let a = class::create(&conn, ClassInput::new("7a", 2025)).unwrap();
    let e = exam::create(&conn, beispiel(vec![a.id])).unwrap();
    let dir = tempfile::tempdir().unwrap();

    let err = export::write_xlsx(&conn, e.id, &dir.path().join("pruefung.txt")).unwrap_err();
    assert_eq!(err.fields().unwrap()[0].field, "path");

    let path = dir.path().join("Prüfung.xlsx");
    export::write_xlsx(&conn, e.id, &path).unwrap();
    let bytes = std::fs::read(&path).unwrap();
    assert_eq!(&bytes[..2], b"PK");
}

#[test]
fn eine_unbekannte_pruefung_ist_nicht_gefunden() {
    let conn = db::open_in_memory().unwrap();
    let err = export::build(&conn, uuid::Uuid::now_v7()).unwrap_err();
    assert!(matches!(err, Error::NotFound));
}

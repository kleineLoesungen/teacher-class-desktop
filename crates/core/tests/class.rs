//! Klassen und Schüler: die fachlichen Regeln.

use teacher_class_core::class::{self, ClassInput};
use teacher_class_core::student::{self, StudentInput};
use teacher_class_core::{Error, db};

fn felder(err: &Error) -> Vec<&'static str> {
    err.fields()
        .expect("Eingabefehler erwartet")
        .iter()
        .map(|f| f.field)
        .collect()
}

#[test]
fn eine_klasse_zeigt_name_und_jahr_zusammen() {
    let conn = db::open_in_memory().unwrap();
    let c = class::create(&conn, ClassInput::new("7a", 2025)).unwrap();
    assert_eq!(c.label(), "7a (2025)");
    assert_eq!(c.student_count, 0);
}

#[test]
fn der_klassenname_erlaubt_nur_buchstaben_und_zahlen() {
    let conn = db::open_in_memory().unwrap();
    for name in ["7 a", "7a!", "", "   ", "7a/b"] {
        let err = class::create(&conn, ClassInput::new(name, 2025)).unwrap_err();
        assert_eq!(felder(&err), ["name"], "Name {name:?}");
    }
}

#[test]
fn das_jahr_muss_vierstellig_sein() {
    let conn = db::open_in_memory().unwrap();
    for year in [999, 10000, 0, -2025] {
        let err = class::create(&conn, ClassInput::new("7a", year)).unwrap_err();
        assert_eq!(felder(&err), ["year"], "Jahr {year}");
    }
}

#[test]
fn dieselbe_klasse_gibt_es_nur_einmal() {
    let conn = db::open_in_memory().unwrap();
    class::create(&conn, ClassInput::new("7a", 2025)).unwrap();
    let err = class::create(&conn, ClassInput::new("7A", 2025)).unwrap_err();
    assert_eq!(felder(&err), ["name"]);
    // anderer Jahrgang ist erlaubt
    class::create(&conn, ClassInput::new("7a", 2026)).unwrap();
}

#[test]
fn klassen_stehen_nach_jahr_absteigend() {
    let conn = db::open_in_memory().unwrap();
    class::create(&conn, ClassInput::new("8b", 2024)).unwrap();
    class::create(&conn, ClassInput::new("7a", 2025)).unwrap();
    let labels: Vec<_> = class::list(&conn)
        .unwrap()
        .iter()
        .map(|c| c.label())
        .collect();
    assert_eq!(labels, ["7a (2025)", "8b (2024)"]);
}

#[test]
fn schueler_entstehen_aus_mehreren_zeilen() {
    let conn = db::open_in_memory().unwrap();
    let c = class::create(&conn, ClassInput::new("7a", 2025)).unwrap();
    let made = student::add_many(&conn, c.id, "Meier, Anna\n\n  Albers , Ben \n").unwrap();
    assert_eq!(made.len(), 2);
    let labels: Vec<_> = student::list(&conn, c.id)
        .unwrap()
        .iter()
        .map(|s| s.label())
        .collect();
    assert_eq!(labels, ["Albers, Ben", "Meier, Anna"]);
    assert_eq!(class::get(&conn, c.id).unwrap().student_count, 2);
}

#[test]
fn ein_fehler_in_einer_zeile_speichert_keinen_schueler() {
    let conn = db::open_in_memory().unwrap();
    let c = class::create(&conn, ClassInput::new("7a", 2025)).unwrap();
    let err =
        student::add_many(&conn, c.id, "Meier, Anna\nOhne Komma\nNoch, Eins, Zwei").unwrap_err();
    assert_eq!(felder(&err), ["students", "students"]);
    let text = err.to_string();
    assert!(text.contains("Zeile 2") || err.fields().unwrap()[0].message.contains("Zeile 2"));
    assert!(student::list(&conn, c.id).unwrap().is_empty());
}

#[test]
fn leere_eingabe_ist_ein_fehler() {
    let conn = db::open_in_memory().unwrap();
    let c = class::create(&conn, ClassInput::new("7a", 2025)).unwrap();
    let err = student::add_many(&conn, c.id, " \n\n").unwrap_err();
    assert_eq!(felder(&err), ["students"]);
}

#[test]
fn schueler_einer_unbekannten_klasse_gibt_not_found() {
    let conn = db::open_in_memory().unwrap();
    let err = student::add_many(&conn, uuid::Uuid::now_v7(), "Meier, Anna").unwrap_err();
    assert!(matches!(err, Error::NotFound));
}

#[test]
fn eine_geloeschte_klasse_nimmt_ihre_schueler_mit() {
    let conn = db::open_in_memory().unwrap();
    let c = class::create(&conn, ClassInput::new("7a", 2025)).unwrap();
    let s = student::add_many(&conn, c.id, "Meier, Anna")
        .unwrap()
        .remove(0);
    class::delete(&conn, c.id).unwrap();
    assert!(matches!(student::get(&conn, s.id), Err(Error::NotFound)));
}

#[test]
fn ein_schueler_laesst_sich_aendern() {
    let conn = db::open_in_memory().unwrap();
    let c = class::create(&conn, ClassInput::new("7a", 2025)).unwrap();
    let s = student::add_many(&conn, c.id, "Meier, Anna")
        .unwrap()
        .remove(0);
    let s = student::update(
        &conn,
        s.id,
        StudentInput {
            last_name: " Maier ".into(),
            first_name: "Anna".into(),
        },
    )
    .unwrap();
    assert_eq!(s.label(), "Maier, Anna");
}

#[test]
fn kurzkennungen_werden_ueber_das_endstueck_aufgeloest() {
    let conn = db::open_in_memory().unwrap();
    let c = class::create(&conn, ClassInput::new("7a", 2025)).unwrap();
    let short = teacher_class_core::ids::short_id(c.id);
    assert_eq!(class::resolve_id(&conn, &short).unwrap(), c.id);
}

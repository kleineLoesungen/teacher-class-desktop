//! Prüfungen: Schema, Bewertungsmatrix und ihre Regeln.

use rusqlite::Connection;
use teacher_class_core::class::{self, ClassInput};
use teacher_class_core::exam::{self, ExamInput, GradeInput, GroupInput, ItemInput};
use teacher_class_core::{Error, db};
use uuid::Uuid;

fn felder(err: &Error) -> Vec<&'static str> {
    err.fields()
        .expect("Eingabefehler erwartet")
        .iter()
        .map(|f| f.field)
        .collect()
}

fn klasse(conn: &Connection, name: &str) -> Uuid {
    class::create(conn, ClassInput::new(name, 2025)).unwrap().id
}

fn item(name: &str, max: f64) -> ItemInput {
    ItemInput {
        name: name.into(),
        max_points: max,
        ..Default::default()
    }
}

fn grade(name: &str, from: f64) -> GradeInput {
    GradeInput {
        name: name.into(),
        from_points: from,
    }
}

/// Zwei Gruppen mit zusammen 20 Punkten und drei Noten.
fn beispiel(class_ids: Vec<Uuid>) -> ExamInput {
    ExamInput {
        name: "Klassenarbeit 1".into(),
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
        with_notes: false,
    }
}

#[test]
fn eine_beschreibung_ist_optional_und_wird_gespeichert() {
    let conn = db::open_in_memory().unwrap();
    let c = klasse(&conn, "7a");
    let mut input = beispiel(vec![c]);
    input.groups[0].items[0].description = "  Rechenweg angeben  ".into();
    let e = exam::create(&conn, input).unwrap();
    assert_eq!(e.groups[0].items[0].description, "Rechenweg angeben");
    assert_eq!(e.groups[0].items[1].description, "");
    assert!(!e.with_notes);
}

#[test]
fn eine_zu_lange_beschreibung_wird_am_feld_abgelehnt() {
    let conn = db::open_in_memory().unwrap();
    let c = klasse(&conn, "7a");
    let mut input = beispiel(vec![c]);
    input.groups[0].items[0].description = "x".repeat(exam::DESCRIPTION_MAX + 1);
    let err = exam::create(&conn, input).unwrap_err();
    assert_eq!(felder(&err), ["groups"]);
}

#[test]
fn begruendungen_werden_bei_der_pruefung_festgelegt() {
    let conn = db::open_in_memory().unwrap();
    let c = klasse(&conn, "7a");
    let mut input = beispiel(vec![c]);
    input.with_notes = true;
    let e = exam::create(&conn, input.clone()).unwrap();
    assert!(e.with_notes);
    input.with_notes = false;
    assert!(!exam::update(&conn, e.id, input).unwrap().with_notes);
}

#[test]
fn maximalpunkte_von_gruppe_und_pruefung_werden_berechnet() {
    let conn = db::open_in_memory().unwrap();
    let c = klasse(&conn, "7a");
    let e = exam::create(&conn, beispiel(vec![c])).unwrap();
    assert_eq!(e.groups[0].max_points, 6.5);
    assert_eq!(e.groups[1].max_points, 13.5);
    assert_eq!(e.max_points, 20.0);
    assert_eq!(e.class_ids, [c]);
}

#[test]
fn die_matrix_ist_aufsteigend_und_bis_wird_berechnet() {
    let conn = db::open_in_memory().unwrap();
    let c = klasse(&conn, "7a");
    let e = exam::create(&conn, beispiel(vec![c])).unwrap();
    let rows: Vec<_> = e
        .grades
        .iter()
        .map(|g| (g.name.as_str(), g.from_points, g.to_points))
        .collect();
    // Halbpunkte: die nächste Note beginnt bei 10, also endet „6“ bei 9,5.
    assert_eq!(
        rows,
        [("6", 0.0, 9.5), ("3", 10.0, 16.5), ("1", 17.0, 20.0)]
    );
}

#[test]
fn halbpunkte_sind_erlaubt_andere_bruchteile_nicht() {
    let conn = db::open_in_memory().unwrap();
    let c = klasse(&conn, "7a");
    let mut input = beispiel(vec![c]);
    input.groups[0].items[0].max_points = 0.3;
    let err = exam::create(&conn, input).unwrap_err();
    assert_eq!(felder(&err), ["groups"]);

    let mut input = beispiel(vec![c]);
    input.groups[0].items[0].max_points = 0.0;
    assert_eq!(felder(&exam::create(&conn, input).unwrap_err()), ["groups"]);
}

#[test]
fn die_niedrigste_note_muss_bei_null_beginnen() {
    let conn = db::open_in_memory().unwrap();
    let c = klasse(&conn, "7a");
    let mut input = beispiel(vec![c]);
    input.grades = vec![grade("1", 17.0), grade("5", 2.0)];
    assert_eq!(felder(&exam::create(&conn, input).unwrap_err()), ["grades"]);
}

#[test]
fn zwei_noten_duerfen_nicht_dasselbe_ab_haben() {
    let conn = db::open_in_memory().unwrap();
    let c = klasse(&conn, "7a");
    let mut input = beispiel(vec![c]);
    input.grades = vec![grade("1", 10.0), grade("2", 10.0), grade("6", 0.0)];
    assert_eq!(felder(&exam::create(&conn, input).unwrap_err()), ["grades"]);
}

#[test]
fn ein_ab_ueber_der_gesamtpunktzahl_wird_abgelehnt() {
    let conn = db::open_in_memory().unwrap();
    let c = klasse(&conn, "7a");
    let mut input = beispiel(vec![c]);
    input.grades = vec![grade("1", 20.5), grade("6", 0.0)];
    assert_eq!(felder(&exam::create(&conn, input).unwrap_err()), ["grades"]);
}

#[test]
fn noten_sind_frei_benennbar() {
    let conn = db::open_in_memory().unwrap();
    let c = klasse(&conn, "7a");
    let mut input = beispiel(vec![c]);
    input.grades = vec![
        grade("1+", 19.0),
        grade("1", 17.0),
        grade("ungenügend", 0.0),
    ];
    let e = exam::create(&conn, input).unwrap();
    assert_eq!(e.grades[0].name, "ungenügend");
    assert_eq!(e.grades[2].name, "1+");
}

#[test]
fn name_klassen_gruppen_und_punkte_sind_pflicht() {
    let conn = db::open_in_memory().unwrap();
    let c = klasse(&conn, "7a");

    let mut input = beispiel(vec![c]);
    input.name = "   ".into();
    input.class_ids.clear();
    input.groups.clear();
    input.grades.clear();
    let err = exam::create(&conn, input).unwrap_err();
    assert_eq!(felder(&err), ["name", "classes", "groups", "grades"]);

    let mut input = beispiel(vec![c]);
    input.groups[0].name = " ".into();
    input.groups[1].items.clear();
    let err = exam::create(&conn, input).unwrap_err();
    assert_eq!(felder(&err), ["groups", "groups"]);
}

#[test]
fn eine_unbekannte_klasse_wird_als_eingabefehler_gemeldet() {
    let conn = db::open_in_memory().unwrap();
    let err = exam::create(&conn, beispiel(vec![Uuid::now_v7()])).unwrap_err();
    assert_eq!(felder(&err), ["classes"]);
}

#[test]
fn eine_pruefung_gilt_fuer_mehrere_klassen() {
    let conn = db::open_in_memory().unwrap();
    let a = klasse(&conn, "7a");
    let b = klasse(&conn, "7b");
    let e = exam::create(&conn, beispiel(vec![a, b, a])).unwrap();
    assert_eq!(e.class_ids.len(), 2, "doppelte Klasse zählt einmal");
}

#[test]
fn aendern_ersetzt_schema_und_matrix_ganz() {
    let conn = db::open_in_memory().unwrap();
    let a = klasse(&conn, "7a");
    let b = klasse(&conn, "7b");
    let e = exam::create(&conn, beispiel(vec![a])).unwrap();

    let mut input = beispiel(vec![b]);
    input.name = "Test".into();
    input.groups = vec![GroupInput {
        name: "Alles".into(),
        items: vec![item("1", 10.0)],
    }];
    input.grades = vec![grade("bestanden", 5.0), grade("nicht bestanden", 0.0)];
    let updated = exam::update(&conn, e.id, input).unwrap();

    assert_eq!(updated.id, e.id);
    assert_eq!(updated.name, "Test");
    assert_eq!(updated.class_ids, [b]);
    assert_eq!(updated.groups.len(), 1);
    assert_eq!(updated.max_points, 10.0);
    assert_eq!(updated.grades[0].to_points, 4.5);
}

#[test]
fn ein_fehlgeschlagenes_aendern_laesst_den_alten_stand() {
    let conn = db::open_in_memory().unwrap();
    let a = klasse(&conn, "7a");
    let e = exam::create(&conn, beispiel(vec![a])).unwrap();
    let mut input = beispiel(vec![a]);
    input.grades.clear();
    exam::update(&conn, e.id, input).unwrap_err();
    assert_eq!(exam::get(&conn, e.id).unwrap(), e);
}

#[test]
fn loeschen_entfernt_alles_und_die_klasse_bleibt() {
    let conn = db::open_in_memory().unwrap();
    let a = klasse(&conn, "7a");
    let e = exam::create(&conn, beispiel(vec![a])).unwrap();
    exam::delete(&conn, e.id).unwrap();
    assert!(matches!(exam::get(&conn, e.id), Err(Error::NotFound)));
    assert!(matches!(exam::delete(&conn, e.id), Err(Error::NotFound)));
    assert!(class::get(&conn, a).is_ok());
    let rest: i64 = conn
        .query_row("SELECT count(*) FROM exam_items", [], |r| r.get(0))
        .unwrap();
    assert_eq!(rest, 0);
}

#[test]
fn eine_geloeschte_klasse_verschwindet_aus_der_pruefung() {
    let conn = db::open_in_memory().unwrap();
    let a = klasse(&conn, "7a");
    let b = klasse(&conn, "7b");
    let e = exam::create(&conn, beispiel(vec![a, b])).unwrap();
    class::delete(&conn, a).unwrap();
    assert_eq!(exam::get(&conn, e.id).unwrap().class_ids, [b]);
}

#[test]
fn die_kurzkennung_loest_eine_pruefung_auf() {
    let conn = db::open_in_memory().unwrap();
    let a = klasse(&conn, "7a");
    let e = exam::create(&conn, beispiel(vec![a])).unwrap();
    let short = teacher_class_core::ids::short_id(e.id);
    assert_eq!(exam::resolve_id(&conn, &short).unwrap(), e.id);
}

#[test]
fn die_liste_zeigt_die_neueste_zuerst() {
    let conn = db::open_in_memory().unwrap();
    let a = klasse(&conn, "7a");
    let first = exam::create(&conn, beispiel(vec![a])).unwrap();
    std::thread::sleep(std::time::Duration::from_millis(5));
    let mut input = beispiel(vec![a]);
    input.name = "Zweite".into();
    let second = exam::create(&conn, input).unwrap();
    let ids: Vec<_> = exam::list(&conn).unwrap().iter().map(|e| e.id).collect();
    assert_eq!(ids, [second.id, first.id]);
}

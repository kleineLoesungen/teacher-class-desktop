//! Bewertung: Punkte und Begründungen je Schüler und Prüfkriterium.

use rusqlite::Connection;
use teacher_class_core::class::{self, ClassInput};
use teacher_class_core::exam::{self, ExamInput, GradeInput, GroupInput, ItemInput};
use teacher_class_core::grading::{self, ScoreInput};
use teacher_class_core::{Error, db, student};
use uuid::Uuid;

fn felder(err: &Error) -> Vec<&'static str> {
    err.fields()
        .expect("Eingabefehler erwartet")
        .iter()
        .map(|f| f.field)
        .collect()
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
fn beispiel(class_ids: Vec<Uuid>, with_notes: bool) -> ExamInput {
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
        with_notes,
    }
}

struct Aufbau {
    conn: Connection,
    exam: Uuid,
    class: Uuid,
    max: Uuid,
    eva: Uuid,
}

fn aufbau(with_notes: bool) -> Aufbau {
    let conn = db::open_in_memory().unwrap();
    let c = class::create(&conn, ClassInput::new("7a", 2025)).unwrap();
    let s = student::add_many(&conn, c.id, "Muster, Max\nBeispiel, Eva").unwrap();
    // Die Liste ist nach Namen sortiert; Zuordnung über den Namen.
    let id_of = |last: &str| s.iter().find(|x| x.last_name == last).unwrap().id;
    let e = exam::create(&conn, beispiel(vec![c.id], with_notes)).unwrap();
    Aufbau {
        exam: e.id,
        class: c.id,
        max: id_of("Muster"),
        eva: id_of("Beispiel"),
        conn,
    }
}

fn eintrag(student_id: Uuid, group: u32, item: u32, points: Option<f64>, note: &str) -> ScoreInput {
    ScoreInput {
        student_id,
        group,
        item,
        points,
        note: note.into(),
    }
}

#[test]
fn ohne_eintraege_gibt_es_weder_summe_noch_note() {
    let a = aufbau(false);
    let results = grading::results(&a.conn, a.exam, a.class).unwrap();
    assert_eq!(results.len(), 2);
    for r in &results {
        assert_eq!(r.total, 0.0);
        assert_eq!(r.grade, None);
        assert!(r.groups[0].scores.iter().all(|s| s.points.is_none()));
    }
}

#[test]
fn summen_und_note_werden_berechnet() {
    let a = aufbau(false);
    grading::set(&a.conn, a.exam, eintrag(a.max, 0, 0, Some(4.0), "")).unwrap();
    grading::set(&a.conn, a.exam, eintrag(a.max, 0, 1, Some(2.5), "")).unwrap();
    let r = grading::set(&a.conn, a.exam, eintrag(a.max, 1, 0, Some(11.0), "")).unwrap();
    assert_eq!(r.groups[0].points, 6.5);
    assert_eq!(r.groups[1].points, 11.0);
    assert_eq!(r.total, 17.5);
    // 17,5 liegt über dem „ab“ der Note 1 (17).
    assert_eq!(r.grade.as_deref(), Some("1"));
}

#[test]
fn null_punkte_sind_eingetragen_und_geben_die_niedrigste_note() {
    let a = aufbau(false);
    let r = grading::set(&a.conn, a.exam, eintrag(a.max, 0, 0, Some(0.0), "")).unwrap();
    assert_eq!(r.groups[0].scores[0].points, Some(0.0));
    assert_eq!(r.grade.as_deref(), Some("6"));
}

#[test]
fn die_note_an_der_grenze_gehoert_zur_besseren() {
    let a = aufbau(false);
    grading::set(&a.conn, a.exam, eintrag(a.max, 1, 0, Some(10.0), "")).unwrap();
    let r = grading::results(&a.conn, a.exam, a.class).unwrap();
    let max = r.iter().find(|r| r.student.id == a.max).unwrap();
    assert_eq!(max.grade.as_deref(), Some("3"));
    // Der andere Schüler ist unberührt.
    let eva = r.iter().find(|r| r.student.id == a.eva).unwrap();
    assert_eq!(eva.grade, None);
}

#[test]
fn leere_punkte_ohne_begruendung_loeschen_den_eintrag() {
    let a = aufbau(false);
    grading::set(&a.conn, a.exam, eintrag(a.max, 0, 0, Some(3.0), "")).unwrap();
    let r = grading::set(&a.conn, a.exam, eintrag(a.max, 0, 0, None, "")).unwrap();
    assert_eq!(r.groups[0].scores[0].points, None);
    assert_eq!(r.grade, None);
}

#[test]
fn punkte_muessen_halbe_zahlen_im_bereich_sein() {
    let a = aufbau(false);
    for bad in [-0.5, 4.5, 1.3, f64::NAN] {
        let err = grading::set(&a.conn, a.exam, eintrag(a.max, 0, 0, Some(bad), "")).unwrap_err();
        assert_eq!(felder(&err), ["points"], "{bad}");
    }
    assert!(grading::set(&a.conn, a.exam, eintrag(a.max, 0, 0, Some(3.5), "")).is_ok());
}

#[test]
fn ein_unbekanntes_pruefkriterium_wird_am_feld_abgelehnt() {
    let a = aufbau(false);
    let err = grading::set(&a.conn, a.exam, eintrag(a.max, 5, 0, Some(1.0), "")).unwrap_err();
    assert_eq!(felder(&err), ["item"]);
    let err = grading::set(&a.conn, a.exam, eintrag(a.max, 0, 2, Some(1.0), "")).unwrap_err();
    assert_eq!(felder(&err), ["item"]);
}

#[test]
fn begruendungen_gibt_es_nur_wenn_die_pruefung_sie_vorsieht() {
    let ohne = aufbau(false);
    let err = grading::set(
        &ohne.conn,
        ohne.exam,
        eintrag(ohne.max, 0, 0, Some(1.0), "Fehler"),
    )
    .unwrap_err();
    assert_eq!(felder(&err), ["note"]);

    let mit = aufbau(true);
    let r = grading::set(
        &mit.conn,
        mit.exam,
        eintrag(mit.max, 0, 0, Some(1.0), "  Fehler  "),
    )
    .unwrap();
    assert_eq!(r.groups[0].scores[0].note, "Fehler");
}

#[test]
fn eine_begruendung_darf_ohne_punkte_stehen() {
    let a = aufbau(true);
    let r = grading::set(&a.conn, a.exam, eintrag(a.max, 0, 0, None, "Noch offen")).unwrap();
    assert_eq!(r.groups[0].scores[0].points, None);
    assert_eq!(r.groups[0].scores[0].note, "Noch offen");
    assert_eq!(r.grade, None);
}

#[test]
fn eine_zu_lange_begruendung_wird_abgelehnt() {
    let a = aufbau(true);
    let long = "x".repeat(grading::NOTE_MAX + 1);
    let err = grading::set(&a.conn, a.exam, eintrag(a.max, 0, 0, Some(1.0), &long)).unwrap_err();
    assert_eq!(felder(&err), ["note"]);
}

#[test]
fn ein_schueler_fremder_klassen_ist_nicht_gefunden() {
    let a = aufbau(false);
    let other = class::create(&a.conn, ClassInput::new("8b", 2025)).unwrap();
    let fremd = student::add_many(&a.conn, other.id, "Fremd, Fritz").unwrap();
    let err = grading::set(&a.conn, a.exam, eintrag(fremd[0].id, 0, 0, Some(1.0), "")).unwrap_err();
    assert!(matches!(err, Error::NotFound));
    let err = grading::results(&a.conn, a.exam, other.id).unwrap_err();
    assert!(matches!(err, Error::NotFound));
}

#[test]
fn beim_aendern_der_pruefung_bleiben_passende_eintraege() {
    let a = aufbau(true);
    grading::set(&a.conn, a.exam, eintrag(a.max, 0, 0, Some(3.0), "gut")).unwrap();
    grading::set(&a.conn, a.exam, eintrag(a.max, 1, 0, Some(13.0), "")).unwrap();

    // Nur den Namen ändern: nichts geht verloren.
    let mut input = beispiel(vec![a.class], true);
    input.name = "Umbenannt".into();
    exam::update(&a.conn, a.exam, input).unwrap();
    let r = grading::results(&a.conn, a.exam, a.class).unwrap();
    let max = r.iter().find(|r| r.student.id == a.max).unwrap();
    assert_eq!(max.total, 16.0);
    assert_eq!(max.groups[0].scores[0].note, "gut");
}

#[test]
fn beim_aendern_der_pruefung_entfallen_nicht_mehr_passende_eintraege() {
    let a = aufbau(true);
    grading::set(&a.conn, a.exam, eintrag(a.max, 0, 0, Some(4.0), "gut")).unwrap();
    grading::set(&a.conn, a.exam, eintrag(a.max, 0, 1, Some(2.5), "")).unwrap();
    grading::set(&a.conn, a.exam, eintrag(a.max, 1, 0, Some(13.0), "")).unwrap();

    // Aufgabe 1/a nur noch 3 Punkte, 1/b entfällt, Aufgabe 2 entfällt.
    let mut input = beispiel(vec![a.class], true);
    input.groups[0].items = vec![item("a", 3.0)];
    input.groups.truncate(1);
    input.grades = vec![grade("1", 3.0), grade("6", 0.0)];
    exam::update(&a.conn, a.exam, input).unwrap();

    let r = grading::results(&a.conn, a.exam, a.class).unwrap();
    let max = r.iter().find(|r| r.student.id == a.max).unwrap();
    // Punkte über der neuen Höchstzahl sind zurückgenommen, die Begründung bleibt.
    assert_eq!(max.groups[0].scores[0].points, None);
    assert_eq!(max.groups[0].scores[0].note, "gut");
    assert_eq!(max.total, 0.0);
    assert_eq!(max.grade, None);
}

#[test]
fn schaltet_man_begruendungen_ab_entfallen_sie() {
    let a = aufbau(true);
    grading::set(&a.conn, a.exam, eintrag(a.max, 0, 0, Some(3.0), "gut")).unwrap();
    grading::set(&a.conn, a.exam, eintrag(a.max, 0, 1, None, "nur Text")).unwrap();
    exam::update(&a.conn, a.exam, beispiel(vec![a.class], false)).unwrap();

    let r = grading::results(&a.conn, a.exam, a.class).unwrap();
    let max = r.iter().find(|r| r.student.id == a.max).unwrap();
    assert_eq!(max.groups[0].scores[0].points, Some(3.0));
    assert_eq!(max.groups[0].scores[0].note, "");
    assert_eq!(max.groups[0].scores[1].note, "");
}

#[test]
fn schueler_entfernter_klassen_verlieren_ihre_eintraege() {
    let a = aufbau(false);
    let other = class::create(&a.conn, ClassInput::new("8b", 2025)).unwrap();
    grading::set(&a.conn, a.exam, eintrag(a.max, 0, 0, Some(3.0), "")).unwrap();
    // Die Prüfung gilt jetzt nur noch für 8b.
    exam::update(&a.conn, a.exam, beispiel(vec![other.id], false)).unwrap();
    let n: i64 = a
        .conn
        .query_row("SELECT count(*) FROM exam_scores", [], |r| r.get(0))
        .unwrap();
    assert_eq!(n, 0);
}

#[test]
fn loescht_man_einen_schueler_verschwinden_seine_eintraege() {
    let a = aufbau(false);
    grading::set(&a.conn, a.exam, eintrag(a.max, 0, 0, Some(3.0), "")).unwrap();
    student::delete(&a.conn, a.max).unwrap();
    let n: i64 = a
        .conn
        .query_row("SELECT count(*) FROM exam_scores", [], |r| r.get(0))
        .unwrap();
    assert_eq!(n, 0);
}

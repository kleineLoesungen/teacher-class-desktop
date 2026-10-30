//! Fachliche Tests der Beispielressource.
//!
//! Hier liegt der Schwerpunkt der Testabdeckung. Die Schalen werden später nur
//! noch daraufhin geprüft, dass sie richtig durchreichen — die Regeln selbst
//! sind hier festgehalten, einmal für beide.

use starter_core::App;
use starter_core::note::{self, NoteInput, Order, TITLE_MAX};

fn app() -> App {
    App::in_memory().expect("Datenbank im Arbeitsspeicher")
}

#[test]
fn legt_eine_notiz_an_und_findet_sie_wieder() {
    let app = app();
    let created = note::create(app.conn(), NoteInput::new("Einkauf", "Milch")).unwrap();

    assert_eq!(created.title, "Einkauf");
    assert_eq!(created.body, "Milch");
    assert_eq!(created.created_at, created.updated_at);

    let found = note::get(app.conn(), created.id).unwrap();
    assert_eq!(found, created);
    assert_eq!(note::count(app.conn()).unwrap(), 1);
}

#[test]
fn schneidet_umgebende_leerzeichen_ab() {
    let app = app();
    let created = note::create(app.conn(), NoteInput::new("  Einkauf  ", "Milch\n\n")).unwrap();
    assert_eq!(created.title, "Einkauf");
    assert_eq!(created.body, "Milch");
}

#[test]
fn ein_titel_aus_leerzeichen_gilt_als_leer() {
    let app = app();
    let err = note::create(app.conn(), NoteInput::new("   ", "Text")).unwrap_err();
    let fields = err.fields().expect("Eingabefehler");
    assert_eq!(fields.len(), 1);
    assert_eq!(fields[0].field, "title");
}

#[test]
fn lehnt_zu_langen_titel_ab() {
    let app = app();
    let long = "ä".repeat(TITLE_MAX + 1);
    let err = note::create(app.conn(), NoteInput::new(long, "")).unwrap_err();
    assert_eq!(err.fields().unwrap()[0].field, "title");
}

#[test]
fn zaehlt_zeichen_nicht_bytes() {
    // "ä" sind zwei Bytes. Eine Byte-Zählung würde hier fälschlich ablehnen.
    let app = app();
    let gerade_noch = "ä".repeat(TITLE_MAX);
    assert!(note::create(app.conn(), NoteInput::new(gerade_noch, "")).is_ok());
}

#[test]
fn meldet_alle_fehlerhaften_felder_auf_einmal() {
    // Wer beim ersten Fehler abbricht, zwingt den Benutzer, dieselbe Maske
    // mehrfach abzuschicken.
    let app = app();
    let err = note::create(
        app.conn(),
        NoteInput::new("", "x".repeat(note::BODY_MAX + 1)),
    )
    .unwrap_err();
    let fields = err.fields().unwrap();
    assert_eq!(fields.len(), 2);
    assert!(fields.iter().any(|f| f.field == "title"));
    assert!(fields.iter().any(|f| f.field == "body"));
}

#[test]
fn unbekannte_id_ist_nicht_gefunden() {
    let app = app();
    let err = note::get(app.conn(), uuid::Uuid::now_v7()).unwrap_err();
    assert!(matches!(err, starter_core::Error::NotFound));
}

#[test]
fn aendern_setzt_den_aenderungszeitpunkt_neu() {
    let app = app();
    let created = note::create(app.conn(), NoteInput::new("Alt", "")).unwrap();
    std::thread::sleep(std::time::Duration::from_millis(5));

    let updated = note::update(app.conn(), created.id, NoteInput::new("Neu", "Text")).unwrap();

    assert_eq!(updated.id, created.id);
    assert_eq!(updated.title, "Neu");
    assert_eq!(updated.created_at, created.created_at, "bleibt unverändert");
    assert!(updated.updated_at > created.updated_at);
}

#[test]
fn aendern_einer_unbekannten_id_legt_nichts_an() {
    let app = app();
    let err = note::update(app.conn(), uuid::Uuid::now_v7(), NoteInput::new("x", "")).unwrap_err();
    assert!(matches!(err, starter_core::Error::NotFound));
    assert_eq!(note::count(app.conn()).unwrap(), 0);
}

#[test]
fn ungueltige_eingabe_aendert_nichts() {
    let app = app();
    let created = note::create(app.conn(), NoteInput::new("Alt", "Text")).unwrap();
    let _ = note::update(app.conn(), created.id, NoteInput::new("", "")).unwrap_err();
    assert_eq!(note::get(app.conn(), created.id).unwrap(), created);
}

#[test]
fn loescht_und_meldet_das_zweite_loeschen_als_nicht_gefunden() {
    let app = app();
    let created = note::create(app.conn(), NoteInput::new("Weg", "")).unwrap();
    note::delete(app.conn(), created.id).unwrap();
    assert_eq!(note::count(app.conn()).unwrap(), 0);
    assert!(matches!(
        note::delete(app.conn(), created.id).unwrap_err(),
        starter_core::Error::NotFound
    ));
}

#[test]
fn sortiert_nach_aenderung_und_nach_titel() {
    let app = app();
    let a = note::create(app.conn(), NoteInput::new("Zebra", "")).unwrap();
    std::thread::sleep(std::time::Duration::from_millis(5));
    let b = note::create(app.conn(), NoteInput::new("Ananas", "")).unwrap();

    let recent = note::list(app.conn(), Order::Recent).unwrap();
    assert_eq!(
        recent.iter().map(|n| n.id).collect::<Vec<_>>(),
        [b.id, a.id]
    );

    let by_title = note::list(app.conn(), Order::Title).unwrap();
    assert_eq!(
        by_title.iter().map(|n| n.id).collect::<Vec<_>>(),
        [b.id, a.id]
    );
}

#[test]
fn sortiert_titel_ohne_ruecksicht_auf_gross_und_klein() {
    let app = app();
    note::create(app.conn(), NoteInput::new("beta", "")).unwrap();
    note::create(app.conn(), NoteInput::new("Alpha", "")).unwrap();
    let titles: Vec<_> = note::list(app.conn(), Order::Title)
        .unwrap()
        .into_iter()
        .map(|n| n.title)
        .collect();
    assert_eq!(titles, ["Alpha", "beta"]);
}

#[test]
fn json_zeigt_zeitstempel_als_text() {
    let app = app();
    let created = note::create(app.conn(), NoteInput::new("Titel", "Text")).unwrap();
    let json = serde_json::to_value(&created).unwrap();

    assert_eq!(json["title"], "Titel");
    let stamp = json["created_at"].as_str().expect("Zeitstempel als Text");
    assert!(stamp.ends_with('Z'), "in UTC gespeichert: {stamp}");
}

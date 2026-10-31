//! Tests des Speicherorts der Datenbank.

use teacher_class_core::class::{self, ClassInput};
use teacher_class_core::{App, Config, Error, ErrorCode, Paths};

#[test]
fn ohne_wahl_liegt_die_datenbank_am_standardort_und_wird_angelegt() {
    let dir = tempfile::tempdir().unwrap();
    let paths = Paths::under(dir.path());

    let app = App::open(paths.clone()).unwrap();

    assert_eq!(app.paths().db_file(), paths.default_db_file());
    assert!(paths.default_db_file().is_file());
}

#[test]
fn eine_neue_datenbank_wird_angelegt_und_beim_naechsten_start_benutzt() {
    let dir = tempfile::tempdir().unwrap();
    let paths = Paths::under(dir.path());
    let ziel = dir.path().join("usb").join("klassen.sqlite3");

    let app = App::create_database(&paths, &ziel).unwrap();
    class::create(app.conn(), ClassInput::new("7a", 2025)).unwrap();
    assert_eq!(app.paths().db_file(), ziel);
    assert!(ziel.is_file());
    drop(app);

    let again = App::open(paths).unwrap();
    assert_eq!(again.paths().db_file(), ziel);
    assert_eq!(class::list(again.conn()).unwrap().len(), 1);
}

#[test]
fn eine_vorhandene_datenbank_wird_uebernommen() {
    let dir = tempfile::tempdir().unwrap();
    let paths = Paths::under(dir.path());
    let ziel = dir.path().join("alt.sqlite3");
    let app = App::create_database(&paths, &ziel).unwrap();
    class::create(app.conn(), ClassInput::new("8b", 2024)).unwrap();
    drop(app);
    App::use_default_database(&paths).unwrap();

    let app = App::use_database(&paths, &ziel).unwrap();

    assert_eq!(class::list(app.conn()).unwrap()[0].name, "8b");
    assert_eq!(app.config().db_file.as_deref(), ziel.to_str());
}

#[test]
fn fehlt_die_datei_am_gewaehlten_ort_wird_nichts_still_angelegt() {
    let dir = tempfile::tempdir().unwrap();
    let paths = Paths::under(dir.path());
    let ziel = dir.path().join("weg.sqlite3");
    drop(App::create_database(&paths, &ziel).unwrap());
    std::fs::remove_file(&ziel).unwrap();

    let err = App::open(paths).err().expect("Fehler erwartet");

    assert!(matches!(err, Error::DatabaseMissing { ref path } if *path == ziel));
    assert_eq!(err.code(), ErrorCode::DatabaseMissing);
    assert!(!ziel.exists(), "es darf keine leere Datei entstanden sein");
}

#[test]
fn zurueck_zum_standardort_vergisst_die_wahl() {
    let dir = tempfile::tempdir().unwrap();
    let paths = Paths::under(dir.path());
    drop(App::create_database(&paths, &dir.path().join("x.sqlite3")).unwrap());

    let app = App::use_default_database(&paths).unwrap();

    assert_eq!(app.paths().db_file(), paths.default_db_file());
    assert_eq!(app.config().db_file, None);
    assert_eq!(Config::load(&paths.config_file()).unwrap().db_file, None);
}

#[test]
fn neu_anlegen_ueberschreibt_nie_eine_vorhandene_datei() {
    let dir = tempfile::tempdir().unwrap();
    let paths = Paths::under(dir.path());
    let ziel = dir.path().join("wichtig.txt");
    std::fs::write(&ziel, "nicht anfassen").unwrap();

    let err = App::create_database(&paths, &ziel).err().unwrap();

    assert_eq!(err.fields().unwrap()[0].field, "db_file");
    assert_eq!(std::fs::read_to_string(&ziel).unwrap(), "nicht anfassen");
}

#[test]
fn eine_fehlende_datei_laesst_sich_nicht_als_vorhandene_waehlen() {
    let dir = tempfile::tempdir().unwrap();
    let paths = Paths::under(dir.path());

    let err = App::use_database(&paths, &dir.path().join("nix.sqlite3"))
        .err()
        .unwrap();

    assert_eq!(err.fields().unwrap()[0].field, "db_file");
}

#[test]
fn eine_fremde_datei_wird_abgelehnt_und_der_alte_ort_bleibt() {
    let dir = tempfile::tempdir().unwrap();
    let paths = Paths::under(dir.path());
    let fremd = dir.path().join("brief.txt");
    std::fs::write(
        &fremd,
        "Das ist keine Datenbank, sondern ein Brief an die Eltern.",
    )
    .unwrap();

    assert!(App::use_database(&paths, &fremd).is_err());

    let app = App::open(paths.clone()).unwrap();
    assert_eq!(app.config().db_file, None);
    assert_eq!(app.paths().db_file(), paths.default_db_file());
}

#[test]
fn ein_relativer_pfad_ist_ein_eingabefehler() {
    let config = Config {
        db_file: Some("daten.sqlite3".into()),
        ..Config::default()
    };

    let err = config.validate().unwrap_err();

    assert_eq!(err.fields().unwrap()[0].field, "db_file");
}

#[test]
fn set_config_aendert_den_speicherort_nicht() {
    // Sonst stünde in der Datei ein Ort, an dem die laufende Anwendung nicht
    // arbeitet.
    let dir = tempfile::tempdir().unwrap();
    let mut app = App::open(Paths::under(dir.path())).unwrap();
    let config = Config {
        db_file: Some(dir.path().join("anders.sqlite3").display().to_string()),
        ..app.config().clone()
    };

    let err = app.set_config(config).unwrap_err();

    assert_eq!(err.fields().unwrap()[0].field, "db_file");
}

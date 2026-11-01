//! Druckvorlagen: Ordner neben der Datenbank, Standarddateien, Dateinamen.

use teacher_class_core::template::{self, Kind};
use teacher_class_core::{App, Paths};

#[test]
fn der_ordner_liegt_neben_der_datenbank_und_wandert_mit() {
    let root = tempfile::tempdir().unwrap();
    let paths = Paths::under(root.path());
    assert_eq!(
        paths.templates_dir(),
        root.path().join("data").join("Templates")
    );

    let elsewhere = root.path().join("stick").join("klassen.sqlite3");
    let moved = paths.clone().with_db_file(Some(elsewhere));
    assert_eq!(
        moved.templates_dir(),
        root.path().join("stick").join("Templates")
    );
}

#[test]
fn beim_oeffnen_und_beim_wechsel_entstehen_die_standards_am_jeweiligen_ort() {
    let root = tempfile::tempdir().unwrap();
    let base = Paths::under(root.path());

    let app = App::open(base.clone()).unwrap();
    let dir = app.paths().templates_dir();
    for file in [
        "print_test_student_standard.md",
        "print_test_class_standard.md",
        "placeholder.md",
    ] {
        assert!(
            dir.join(file).is_file(),
            "{file} fehlt in {}",
            dir.display()
        );
    }

    // Neuer Ort: dort liegt noch nichts, also kommen die Standards dorthin.
    let file = root.path().join("stick").join("neu.sqlite3");
    std::fs::create_dir_all(file.parent().unwrap()).unwrap();
    let app = App::create_database(&base, &file).unwrap();
    let dir = app.paths().templates_dir();
    assert_eq!(dir, root.path().join("stick").join("Templates"));
    assert!(dir.join("print_test_student_standard.md").is_file());
}

#[test]
fn eine_vorhandene_vorlage_wird_nie_ueberschrieben_die_platzhalterliste_schon() {
    let dir = tempfile::tempdir().unwrap();
    let custom = dir.path().join("print_test_student_standard.md");
    std::fs::write(&custom, "meine Fassung").unwrap();
    std::fs::write(dir.path().join("placeholder.md"), "veraltet").unwrap();

    template::ensure(dir.path()).unwrap();
    assert_eq!(std::fs::read_to_string(&custom).unwrap(), "meine Fassung");
    let doc = std::fs::read_to_string(dir.path().join("placeholder.md")).unwrap();
    assert_ne!(doc, "veraltet");
    assert_eq!(doc, template::placeholder_doc());
}

#[test]
fn die_platzhalterliste_nennt_alles_was_es_gibt() {
    let doc = template::placeholder_doc();
    for name in [
        "{{pruefung}}",
        "{{schueler}}",
        "{{gesamtpunkte}}",
        "{{#gruppen}}",
        "{{#aufgaben}}",
        "{{begruendung}}",
        "{{#noten}}",
        "{{anzahl}}",
        "{{#schueler}}",
        "{{#wenn name}}",
        "print_test_student_NAME.md",
        "print_test_class_NAME.md",
    ] {
        assert!(doc.contains(name), "{name} fehlt in placeholder.md");
    }
}

#[test]
fn die_liste_trennt_nach_dateiname_und_stellt_standards_voran() {
    let dir = tempfile::tempdir().unwrap();
    for f in [
        "print_test_student_zeugnis.md",
        "print_test_student_ausdruck.md",
        "print_test_class_uebersicht.md",
        "notizen.md",
        "print_test_student_.md",
        "print_test_class_x.txt",
    ] {
        std::fs::write(dir.path().join(f), "x").unwrap();
    }

    let list = template::list(dir.path()).unwrap();
    let got: Vec<(&str, Kind, bool)> = list
        .templates
        .iter()
        .map(|t| (t.file.as_str(), t.kind, t.standard))
        .collect();
    assert_eq!(
        got,
        vec![
            ("print_test_student_standard.md", Kind::Student, true),
            ("print_test_student_ausdruck.md", Kind::Student, false),
            ("print_test_student_zeugnis.md", Kind::Student, false),
            ("print_test_class_standard.md", Kind::Class, true),
            ("print_test_class_uebersicht.md", Kind::Class, false),
        ]
    );
    assert_eq!(list.dir, dir.path().to_string_lossy());
}

#[test]
fn die_standarddateien_sind_gueltige_vorlagen() {
    for kind in [Kind::Student, Kind::Class] {
        let source = template::standard_source(kind);
        let file = match kind {
            Kind::Student => template::STUDENT_STANDARD_FILE,
            Kind::Class => template::CLASS_STANDARD_FILE,
        };
        let tpl = template::Template {
            file: file.into(),
            kind,
            source,
        };
        template::compile(&tpl).unwrap();
    }
}

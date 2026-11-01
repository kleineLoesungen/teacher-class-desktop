//! PDF-Ausdruck: eine Seite je Schüler, ein PDF je Klasse — gesetzt aus einer
//! Druckvorlage.
//!
//! Gesehen hat die Seiten hier niemand; die Tests prüfen Aufbau, Seitenzahl und
//! dass die Texte im Inhaltsstrom stehen (Hex-Zeichenketten in WinAnsi).

use std::path::Path;

use rusqlite::Connection;
use teacher_class_core::class::{self, ClassInput};
use teacher_class_core::exam::{self, ExamInput, GradeInput, GroupInput, ItemInput};
use teacher_class_core::grading::{self, ScoreInput};
use teacher_class_core::{Error, db, export_pdf, student};

const STUDENT: &str = "print_test_student_standard.md";
const CLASS: &str = "print_test_class_standard.md";

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
                'Ä' => 0xC4,
                c => c as u8,
            };
            format!("{byte:02X}")
        })
        .collect()
}

fn text(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes).into_owned()
}

fn pages(pdf: &str) -> usize {
    pdf.matches("/Type /Page /").count()
}

fn build(conn: &Connection, dir: &Path, e: uuid::Uuid, c: uuid::Uuid, file: &str) -> String {
    text(&export_pdf::build(conn, dir, e, c, file).unwrap())
}

/// Die Meldung eines Vorlagenfehlers; prüft, dass es ein Eingabefehler am
/// Feld `template` ist.
fn template_error(err: Error) -> String {
    let fields = err.fields().expect("ein Eingabefehler");
    assert_eq!(fields[0].field, "template");
    fields[0].message.clone()
}

#[test]
fn ein_pdf_je_klasse_mit_einer_seite_je_schueler() {
    let conn = db::open_in_memory().unwrap();
    let dir = tempfile::tempdir().unwrap();
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

    let pdf = build(&conn, dir.path(), e.id, a.id, STUDENT);
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
        assert!(pdf.contains(&hex(expected)), "{expected} fehlt");
    }
    // Die andere Klasse ist nicht dabei.
    assert!(!pdf.contains(&hex("Test, Tom")));

    let other = build(&conn, dir.path(), e.id, b.id, STUDENT);
    assert_eq!(pages(&other), 1);
    assert!(other.contains(&hex("Test, Tom")));
}

#[test]
fn die_begruendung_steht_nur_bei_pruefungen_mit_begruendungen() {
    let conn = db::open_in_memory().unwrap();
    let dir = tempfile::tempdir().unwrap();
    let a = class::create(&conn, ClassInput::new("7a", 2025)).unwrap();
    student::add_many(&conn, a.id, "Muster, Max").unwrap();
    let mut input = beispiel(vec![a.id]);
    input.with_notes = false;
    let e = exam::create(&conn, input).unwrap();

    let pdf = build(&conn, dir.path(), e.id, a.id, STUDENT);
    assert!(!pdf.contains(&hex("Begründung")));
    assert_eq!(pages(&pdf), 1);
}

#[test]
fn eine_sehr_lange_begruendung_laeuft_auf_die_naechste_seite() {
    let conn = db::open_in_memory().unwrap();
    let dir = tempfile::tempdir().unwrap();
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

    let pdf = build(&conn, dir.path(), e.id, a.id, STUDENT);
    // Max Muster braucht mehr als eine Seite, Eva Beispiel beginnt trotzdem neu.
    assert!(pages(&pdf) >= 3, "{} Seiten", pages(&pdf));
    // Der Tabellenkopf wiederholt sich auf der Folgeseite.
    assert!(pdf.matches(&hex("Aufgabe")).count() >= 4);
}

#[test]
fn eine_klasse_ohne_schueler_ergibt_eine_seite() {
    let conn = db::open_in_memory().unwrap();
    let dir = tempfile::tempdir().unwrap();
    let a = class::create(&conn, ClassInput::new("7a", 2025)).unwrap();
    let e = exam::create(&conn, beispiel(vec![a.id])).unwrap();

    let pdf = build(&conn, dir.path(), e.id, a.id, STUDENT);
    assert_eq!(pages(&pdf), 1);
    assert!(pdf.contains(&hex("Keine Schüler in dieser Klasse.")));
}

#[test]
fn zeichen_ausserhalb_von_winansi_werden_zu_fragezeichen() {
    let conn = db::open_in_memory().unwrap();
    let dir = tempfile::tempdir().unwrap();
    let a = class::create(&conn, ClassInput::new("7a", 2025)).unwrap();
    student::add_many(&conn, a.id, "Łukasz, Ola").unwrap();
    let e = exam::create(&conn, beispiel(vec![a.id])).unwrap();

    let pdf = build(&conn, dir.path(), e.id, a.id, STUDENT);
    assert!(pdf.contains(&hex("?ukasz, Ola")));
}

#[test]
fn eine_klasse_ausserhalb_der_pruefung_wird_abgelehnt() {
    let conn = db::open_in_memory().unwrap();
    let dir = tempfile::tempdir().unwrap();
    let a = class::create(&conn, ClassInput::new("7a", 2025)).unwrap();
    let b = class::create(&conn, ClassInput::new("7b", 2025)).unwrap();
    let e = exam::create(&conn, beispiel(vec![a.id])).unwrap();

    let err = export_pdf::build(&conn, dir.path(), e.id, b.id, STUDENT).unwrap_err();
    assert_eq!(err.fields().unwrap()[0].field, "class");
    let err =
        export_pdf::build(&conn, dir.path(), uuid::Uuid::now_v7(), a.id, STUDENT).unwrap_err();
    assert!(matches!(err, Error::NotFound));
}

#[test]
fn speichern_verlangt_pdf_und_schreibt_die_datei() {
    let conn = db::open_in_memory().unwrap();
    let a = class::create(&conn, ClassInput::new("7a", 2025)).unwrap();
    let e = exam::create(&conn, beispiel(vec![a.id])).unwrap();
    let dir = tempfile::tempdir().unwrap();
    let out = dir.path().join("out");
    std::fs::create_dir(&out).unwrap();
    let tpl = dir.path().join("Templates");

    let err =
        export_pdf::write_pdf(&conn, &tpl, e.id, a.id, STUDENT, &out.join("a.txt")).unwrap_err();
    assert_eq!(err.fields().unwrap()[0].field, "path");

    let path = out.join("Prüfung.pdf");
    export_pdf::write_pdf(&conn, &tpl, e.id, a.id, STUDENT, &path).unwrap();
    assert_eq!(&std::fs::read(&path).unwrap()[..5], b"%PDF-");
}

// --- Vorlagen ----------------------------------------------------------------

/// Eine Klasse mit drei Schülern und einer Prüfung.
fn klasse() -> (Connection, tempfile::TempDir, uuid::Uuid, uuid::Uuid) {
    let conn = db::open_in_memory().unwrap();
    let a = class::create(&conn, ClassInput::new("7a", 2025)).unwrap();
    let s = student::add_many(&conn, a.id, "Muster, Max\nBeispiel, Eva\nZweig, Uwe").unwrap();
    let e = exam::create(&conn, beispiel(vec![a.id])).unwrap();
    for (n, st) in s.iter().enumerate() {
        grading::set(
            &conn,
            e.id,
            ScoreInput {
                student_id: st.id,
                group: 1,
                item: 0,
                points: Some(if n == 0 { 13.5 } else { 4.0 }),
                note: String::new(),
            },
        )
        .unwrap();
    }
    (conn, tempfile::tempdir().unwrap(), e.id, a.id)
}

#[test]
fn die_klassenvorlage_ergibt_ein_dokument_mit_allen_schuelern() {
    let (conn, dir, e, a) = klasse();
    let pdf = build(&conn, dir.path(), e, a, CLASS);
    assert_eq!(pages(&pdf), 1);
    for expected in [
        "Muster, Max",
        "Beispiel, Eva",
        "Zweig, Uwe",
        "Notenspiegel",
        "13,5",
    ] {
        assert!(pdf.contains(&hex(expected)), "{expected} fehlt");
    }
}

#[test]
fn eine_eigene_vorlage_wird_je_schueler_ausgefuellt() {
    let (conn, dir, e, a) = klasse();
    std::fs::write(
        dir.path().join("print_test_student_kurz.md"),
        "# {{schueler}}\n\nNote {{note}} bei {{gesamtpunkte}} von {{max_punkte}} Punkten\n\n\
         {{#gruppen}}\n* {{name}}: {{punkte}}\n{{/gruppen}}\n",
    )
    .unwrap();

    let pdf = build(&conn, dir.path(), e, a, "print_test_student_kurz.md");
    assert_eq!(pages(&pdf), 3);
    assert!(pdf.contains(&hex("Note 6 bei 13,5 von 20 Punkten")));
    assert!(pdf.contains(&hex("Aufgabe 2: 13,5")));
}

#[test]
fn eingesetzte_werte_sind_text_und_kein_markdown() {
    let conn = db::open_in_memory().unwrap();
    let dir = tempfile::tempdir().unwrap();
    let a = class::create(&conn, ClassInput::new("7a", 2025)).unwrap();
    student::add_many(&conn, a.id, "Muster, Max").unwrap();
    let mut input = beispiel(vec![a.id]);
    input.groups[0].items[0].description = "**nicht fett** | _kein_ Rand".into();
    let e = exam::create(&conn, input).unwrap();

    let pdf = build(&conn, dir.path(), e.id, a.id, STUDENT);
    assert!(pdf.contains(&hex("**nicht fett** | _kein_ Rand")));
}

#[test]
fn ein_unbekannter_platzhalter_bricht_mit_datei_und_zeile_ab() {
    let (conn, dir, e, a) = klasse();
    let file = "print_test_student_fehler.md";
    std::fs::write(
        dir.path().join(file),
        "# {{schueler}}\n\nHallo\n\n{{vorname_falsch}}\n",
    )
    .unwrap();

    let msg = template_error(export_pdf::build(&conn, dir.path(), e, a, file).unwrap_err());
    assert!(msg.contains(file), "{msg}");
    assert!(msg.contains("Zeile 5"), "{msg}");
    assert!(msg.contains("{{vorname_falsch}}"), "{msg}");
}

#[test]
fn eine_liste_in_der_falschen_ebene_wird_erklaert() {
    let (conn, dir, e, a) = klasse();
    let file = "print_test_student_ebene.md";
    // `aufgaben` gibt es nur innerhalb von `gruppen`.
    std::fs::write(
        dir.path().join(file),
        "{{#aufgaben}}\n{{name}}\n{{/aufgaben}}\n",
    )
    .unwrap();
    let msg = template_error(export_pdf::build(&conn, dir.path(), e, a, file).unwrap_err());
    assert!(msg.contains("Zeile 1") && msg.contains("aufgaben"), "{msg}");

    // `schueler` gibt es nur in Klassenvorlagen.
    std::fs::write(dir.path().join(file), "{{#schueler}}x{{/schueler}}").unwrap();
    let msg = template_error(export_pdf::build(&conn, dir.path(), e, a, file).unwrap_err());
    assert!(msg.contains("schueler"), "{msg}");
}

#[test]
fn nicht_geschlossene_oder_falsch_geschlossene_bloecke_werden_gemeldet() {
    let (conn, dir, e, a) = klasse();
    let file = "print_test_student_bloecke.md";
    for (source, wanted) in [
        ("a\n{{#gruppen}}\nb\n", "Zeile 2"),
        ("{{#gruppen}}\n{{/noten}}\n", "schließt nicht"),
        ("{{/gruppen}}", "ohne Anfang"),
        ("{{#sonst}}", "{{#sonst}}"),
        ("{{schueler", "ohne schließendes"),
        ("<style>\nbody { }\n", "</style>"),
    ] {
        std::fs::write(dir.path().join(file), source).unwrap();
        let msg = template_error(export_pdf::build(&conn, dir.path(), e, a, file).unwrap_err());
        assert!(msg.contains(wanted), "{source:?}: {msg}");
    }
}

#[test]
fn wenn_und_sonst_waehlen_aus() {
    let (conn, dir, e, a) = klasse();
    let file = "print_test_student_wenn.md";
    std::fs::write(
        dir.path().join(file),
        "{{#wenn abschluss_satz}}\nMit Satz\n{{#sonst}}\nOhne Satz\n{{/wenn}}\n",
    )
    .unwrap();
    let pdf = build(&conn, dir.path(), e, a, file);
    assert!(pdf.contains(&hex("Ohne Satz")));
    assert!(!pdf.contains(&hex("Mit Satz")));
}

#[test]
fn css_wirkt_und_ein_fehler_darin_bricht_ab() {
    let (conn, dir, e, a) = klasse();
    let file = "print_test_class_css.md";
    std::fs::write(
        dir.path().join(file),
        "<style>\nbody { margin: 20mm; font-size: 12pt }\nh1 { color: #336699; text-align: center }\n</style>\n# {{pruefung}}\n\nText\n",
    )
    .unwrap();
    let pdf = build(&conn, dir.path(), e, a, file);
    assert!(pdf.contains("0.200 0.400 0.600 rg"), "Farbe fehlt");
    assert!(pdf.contains("12 Tf"));

    for (css, wanted) in [
        ("h1 { colour: red }", "colour"),
        ("blink { color: red }", "blink"),
        ("h1 { color: lila }", "keine Farbe"),
        ("h1 { font-size: groß }", "Größe"),
        ("body { margin: 400pt }", "Seitenrand"),
        ("h1 { color: red", "fehlt"),
    ] {
        std::fs::write(
            dir.path().join(file),
            format!("<style>\n{css}\n</style>\n# x\n"),
        )
        .unwrap();
        let msg = template_error(export_pdf::build(&conn, dir.path(), e, a, file).unwrap_err());
        assert!(msg.contains(file) && msg.contains(wanted), "{css}: {msg}");
    }
}

#[test]
fn die_standardvorlage_geht_immer_auch_ohne_oder_mit_kaputter_datei() {
    let (conn, dir, e, a) = klasse();
    let file = dir.path().join(STUDENT);
    // Ordner leer: wird angelegt.
    assert_eq!(pages(&build(&conn, dir.path(), e, a, STUDENT)), 3);
    assert!(file.is_file());

    // Datei verstümmelt: Die eingebaute Fassung zählt, nicht die Datei.
    std::fs::write(&file, "{{kaputt").unwrap();
    assert_eq!(pages(&build(&conn, dir.path(), e, a, STUDENT)), 3);
    assert_eq!(
        std::fs::read_to_string(&file).unwrap(),
        "{{kaputt",
        "nie überschrieben"
    );

    // Datei gelöscht, Ordner gelöscht: ebenso.
    std::fs::remove_file(&file).unwrap();
    assert_eq!(pages(&build(&conn, dir.path(), e, a, STUDENT)), 3);
    std::fs::remove_dir_all(dir.path()).unwrap();
    assert_eq!(pages(&build(&conn, dir.path(), e, a, CLASS)), 1);
}

#[test]
fn unpassende_dateinamen_und_fehlende_vorlagen_werden_abgelehnt() {
    let (conn, dir, e, a) = klasse();
    for (file, wanted) in [
        ("brief.md", "keine Druckvorlage"),
        ("print_test_student_.md", "keine Druckvorlage"),
        ("../print_test_student_x.md", "keine Druckvorlage"),
        ("print_test_student_x.txt", "keine Druckvorlage"),
        ("print_test_student_gibtsnicht.md", "gibt es nicht"),
    ] {
        let msg = template_error(export_pdf::build(&conn, dir.path(), e, a, file).unwrap_err());
        assert!(msg.contains(wanted), "{file}: {msg}");
    }
}

// --- Bilder --------------------------------------------------------------------

fn png_with_alpha(path: &Path) {
    let file = std::fs::File::create(path).unwrap();
    let mut enc = png::Encoder::new(file, 2, 2);
    enc.set_color(png::ColorType::Rgba);
    enc.set_depth(png::BitDepth::Eight);
    let mut w = enc.write_header().unwrap();
    w.write_image_data(&[
        255, 0, 0, 255, 0, 255, 0, 128, 0, 0, 255, 0, 255, 255, 255, 255,
    ])
    .unwrap();
}

#[test]
fn ein_logo_als_png_mit_transparenz_kommt_ins_pdf() {
    let (conn, dir, e, a) = klasse();
    png_with_alpha(&dir.path().join("logo.png"));
    let file = "print_test_student_logo.md";
    std::fs::write(
        dir.path().join(file),
        "![Logo](logo.png \"30mm\")\n\n# {{schueler}}\n",
    )
    .unwrap();

    let pdf = build(&conn, dir.path(), e, a, file);
    assert_eq!(pages(&pdf), 3);
    assert_eq!(
        pdf.matches("/Subtype /Image").count(),
        2,
        "Bild und Maske, einmal"
    );
    assert!(pdf.contains("/SMask"));
    assert_eq!(pdf.matches("/Im1 Do").count(), 3, "auf jeder Seite");
    // 30 mm = 85,04 pt breit.
    assert!(pdf.contains("q 85.04 0 0 85.04 "), "Größe falsch");
}

#[test]
fn ein_jpeg_geht_unveraendert_ins_pdf() {
    let (conn, dir, e, a) = klasse();
    // Nur Kopf: Startmarke, SOF0 (8 Bit, 4 x 2 Punkte, 3 Farben).
    let jpeg = [
        0xFF, 0xD8, 0xFF, 0xC0, 0x00, 0x11, 0x08, 0x00, 0x02, 0x00, 0x04, 0x03, 0x01, 0x22, 0x00,
        0x02, 0x11, 0x01, 0x03, 0x11, 0x01, 0xFF, 0xD9,
    ];
    std::fs::write(dir.path().join("logo.jpg"), jpeg).unwrap();
    let file = "print_test_class_logo.md";
    std::fs::write(
        dir.path().join(file),
        "<style>img { width: 50% }</style>\n![Logo](logo.jpg)\n",
    )
    .unwrap();

    let pdf = build(&conn, dir.path(), e, a, file);
    assert!(pdf.contains("/Filter /DCTDecode"));
    assert!(pdf.contains("/Width 4 /Height 2"));
}

#[test]
fn ein_fehlendes_oder_unerlaubtes_bild_bricht_ab() {
    let (conn, dir, e, a) = klasse();
    std::fs::write(dir.path().join("kein.png"), "kein Bild").unwrap();
    let file = "print_test_student_bild.md";
    for (src, wanted) in [
        ("fehlt.png", "fehlt.png"),
        ("kein.png", "weder PNG noch JPEG"),
        ("../logo.png", "nur Dateien im Vorlagenordner"),
        ("/etc/logo.png", "nur Dateien im Vorlagenordner"),
        (
            "https://example.org/logo.png",
            "nur Dateien im Vorlagenordner",
        ),
    ] {
        std::fs::write(dir.path().join(file), format!("![Logo]({src})\n")).unwrap();
        let msg = template_error(export_pdf::build(&conn, dir.path(), e, a, file).unwrap_err());
        assert!(msg.contains(file) && msg.contains(wanted), "{src}: {msg}");
    }
}

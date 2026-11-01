//! Druckvorlagen: Markdown-Dateien mit Platzhaltern.
//!
//! Sie liegen im Ordner `Templates` **neben der Datenbank** (siehe
//! [`crate::Paths::templates_dir`]). Wechselt der Speicherort der Datenbank,
//! gelten die Vorlagen dort; fehlen sie, werden die Standards angelegt.
//!
//! # Dateinamen
//!
//! Der Dateiname entscheidet, wofür eine Vorlage gilt:
//!
//! * `print_test_student_NAME.md` — wird **je Schüler** ausgefüllt, jeder
//!   Schüler beginnt auf einer neuen Seite.
//! * `print_test_class_NAME.md` — wird **einmal je Klasse** ausgefüllt
//!   (Übersicht über die Klasse in einer Prüfung).
//!
//! `NAME` = `standard` sind die Vorlagen der App. Sie werden **nie gelesen**:
//! Die App benutzt ihre eingebaute Fassung, damit der Ausdruck immer geht,
//! auch wenn jemand die Datei löscht oder verstümmelt. Die Datei ist nur eine
//! Kopie zum Abschreiben — man legt sie unter anderem Namen ab und ändert sie
//! dort.
//!
//! # Platzhalter
//!
//! `{{name}}` setzt einen Wert ein. `{{#liste}} … {{/liste}}` wiederholt den
//! Block je Eintrag, `{{#wenn name}} … {{#sonst}} … {{/wenn}}` wählt aus.
//! Was es gibt, steht in `Templates/placeholder.md`; die Datei erzeugt
//! [`placeholder_doc`] aus denselben Tabellen, gegen die geprüft wird. Ein
//! unbekannter Platzhalter bricht den Druck ab — er bleibt nie still leer.
//!
//! Eingesetzte Werte sind Text, nie Markdown: Ein Sternchen im Namen eines
//! Schülers macht nichts fett.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use serde::Serialize;
use ts_rs::TS;

use crate::error::{Error, Result, Validator};

/// Name des Ordners neben der Datenbank.
pub const DIR_NAME: &str = "Templates";
/// Die Liste der Platzhalter; wird bei jedem Start neu geschrieben.
pub const PLACEHOLDER_FILE: &str = "placeholder.md";
/// Der `NAME` der eingebauten Vorlagen.
pub const STANDARD: &str = "standard";

const STUDENT_PREFIX: &str = "print_test_student_";
const CLASS_PREFIX: &str = "print_test_class_";

pub const STUDENT_STANDARD_FILE: &str = "print_test_student_standard.md";
pub const CLASS_STANDARD_FILE: &str = "print_test_class_standard.md";

/// Wofür eine Vorlage gilt.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "snake_case")]
#[ts(rename = "TemplateKind")]
pub enum Kind {
    /// Eine Ausfertigung je Schüler.
    Student,
    /// Eine Ausfertigung je Klasse.
    Class,
}

/// Eine Vorlage zur Auswahl im Druckdialog.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
pub struct TemplateInfo {
    /// Dateiname, z. B. `print_test_student_standard.md`.
    pub file: String,
    pub kind: Kind,
    /// Der Teil nach dem Präfix, ohne `.md`: `standard`, `schulkopf`.
    pub name: String,
    /// Die eingebaute Vorlage der App.
    pub standard: bool,
}

/// Die Vorlagen und wo sie liegen.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
pub struct TemplateList {
    /// Der Ordner, in dem man eigene Vorlagen ablegt.
    pub dir: String,
    pub templates: Vec<TemplateInfo>,
}

/// Art und `NAME` aus einem Dateinamen; `None`, wenn er nicht passt.
pub fn parse_file_name(file: &str) -> Option<(Kind, &str)> {
    let stem = file.strip_suffix(".md")?;
    for (prefix, kind) in [(STUDENT_PREFIX, Kind::Student), (CLASS_PREFIX, Kind::Class)] {
        if let Some(name) = stem.strip_prefix(prefix) {
            let ok = !name.is_empty() && !name.contains(['/', '\\']) && name != "." && name != "..";
            return ok.then_some((kind, name));
        }
    }
    None
}

fn io_error(path: &Path) -> impl FnOnce(std::io::Error) -> Error + '_ {
    move |source| Error::Io {
        path: path.to_path_buf(),
        source,
    }
}

/// Legt den Ordner an, schreibt fehlende Standardvorlagen und die Liste der
/// Platzhalter. Eine vorhandene Vorlage wird **nie** überschrieben — nur
/// `placeholder.md` ist Dokumentation und wird aktualisiert.
pub fn ensure(dir: &Path) -> Result<()> {
    std::fs::create_dir_all(dir).map_err(io_error(dir))?;
    for (file, text) in [
        (STUDENT_STANDARD_FILE, standard_source(Kind::Student)),
        (CLASS_STANDARD_FILE, standard_source(Kind::Class)),
    ] {
        let path = dir.join(file);
        match std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&path)
        {
            Ok(mut f) => {
                use std::io::Write as _;
                f.write_all(text.as_bytes()).map_err(io_error(&path))?;
            }
            Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => {}
            Err(e) => return Err(io_error(&path)(e)),
        }
    }
    let path = dir.join(PLACEHOLDER_FILE);
    let doc = placeholder_doc();
    if std::fs::read_to_string(&path).ok().as_deref() != Some(doc.as_str()) {
        std::fs::write(&path, doc).map_err(io_error(&path))?;
    }
    Ok(())
}

/// Alle Vorlagen im Ordner: erst die Standards, dann die eigenen nach Namen.
pub fn list(dir: &Path) -> Result<TemplateList> {
    ensure(dir)?;
    let mut templates = Vec::new();
    for entry in std::fs::read_dir(dir).map_err(io_error(dir))? {
        let entry = entry.map_err(io_error(dir))?;
        let Some(file) = entry.file_name().to_str().map(str::to_owned) else {
            continue;
        };
        if let Some((kind, name)) = parse_file_name(&file) {
            templates.push(TemplateInfo {
                standard: name == STANDARD,
                name: name.to_owned(),
                kind,
                file,
            });
        }
    }
    templates.sort_by(|a, b| {
        (a.kind as u8, !a.standard, a.name.to_lowercase()).cmp(&(
            b.kind as u8,
            !b.standard,
            b.name.to_lowercase(),
        ))
    });
    Ok(TemplateList {
        dir: dir.to_string_lossy().into_owned(),
        templates,
    })
}

/// Eine geladene Vorlage, noch nicht ausgefüllt.
#[derive(Debug, Clone)]
pub struct Template {
    pub file: String,
    pub kind: Kind,
    pub source: String,
}

/// Lädt `file` aus `dir`. Die Standardvorlagen kommen aus dem Programm.
pub fn load(dir: &Path, file: &str) -> Result<Template> {
    let Some((kind, name)) = parse_file_name(file) else {
        return Err(template_error(format!(
            "„{file}“ ist keine Druckvorlage — der Dateiname muss mit {STUDENT_PREFIX} \
             oder {CLASS_PREFIX} beginnen und auf .md enden"
        )));
    };
    if name == STANDARD {
        return Ok(Template {
            file: file.to_owned(),
            kind,
            source: standard_source(kind),
        });
    }
    let path: PathBuf = dir.join(file);
    match std::fs::read_to_string(&path) {
        Ok(source) => Ok(Template {
            file: file.to_owned(),
            kind,
            source,
        }),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Err(template_error(format!(
            "Die Vorlage {file} gibt es nicht in {}",
            dir.display()
        ))),
        Err(e) => Err(template_error(format!(
            "{file} lässt sich nicht lesen: {e}"
        ))),
    }
}

pub fn template_error(message: String) -> Error {
    let mut v = Validator::new();
    v.add("template", message);
    v.finish().expect_err("add schlägt immer fehl")
}

// --- Platzhalter: was es gibt ------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Scope {
    /// Oberste Ebene einer Schüler-Vorlage: allgemeine Werte und ein Schüler.
    RootStudent,
    /// Oberste Ebene einer Klassen-Vorlage.
    RootClass,
    Student,
    Group,
    Item,
    Grade,
}

type Field = (&'static str, &'static str);

const COMMON: &[Field] = &[
    ("pruefung", "Name der Prüfung"),
    ("klasse", "Klasse, z. B. 7a (2025)"),
    ("max_punkte", "Höchstpunktzahl der Prüfung"),
    (
        "begruendungen",
        "„ja“, wenn die Prüfung Begründungen hat, sonst leer",
    ),
    ("datum", "heutiges Datum, z. B. 01.11.2026"),
    ("anzahl_schueler", "Zahl der Schüler der Klasse"),
];
const STUDENT: &[Field] = &[
    ("schueler", "Nachname, Vorname"),
    ("nachname", "Nachname"),
    ("vorname", "Vorname"),
    ("gesamtpunkte", "Gesamtpunkte des Schülers"),
    ("note", "Note; leer, solange nichts bewertet ist"),
    (
        "abschluss_satz",
        "Abschluss-Satz; leer, wenn keiner eingetragen ist",
    ),
];
const GROUP: &[Field] = &[
    ("name", "Name der Gruppe"),
    ("punkte", "erreichte Punkte in der Gruppe"),
    ("max_punkte", "Höchstpunktzahl der Gruppe"),
];
const ITEM: &[Field] = &[
    ("name", "Name des Punktes"),
    (
        "beschreibung",
        "Beschreibung; leer, wenn keine eingetragen ist",
    ),
    ("punkte", "erreichte Punkte; leer, wenn nicht bewertet"),
    ("max_punkte", "Höchstpunktzahl des Punktes"),
    (
        "begruendung",
        "Begründung; leer, wenn keine eingetragen ist",
    ),
];
const GRADE: &[Field] = &[
    ("name", "Name der Note"),
    ("von", "ab dieser Punktzahl"),
    ("bis", "bis zu dieser Punktzahl"),
    ("anzahl", "Zahl der Schüler der Klasse mit dieser Note"),
];

/// Liste: Name, Beschreibung, Eintragsart.
type ListDef = (&'static str, &'static str, Scope);

fn scalars(scope: Scope) -> Vec<&'static [Field]> {
    match scope {
        Scope::RootStudent => vec![COMMON, STUDENT],
        Scope::RootClass => vec![COMMON],
        Scope::Student => vec![STUDENT],
        Scope::Group => vec![GROUP],
        Scope::Item => vec![ITEM],
        Scope::Grade => vec![GRADE],
    }
}

fn lists(scope: Scope) -> &'static [ListDef] {
    const GRADES: ListDef = (
        "noten",
        "die Bewertungsmatrix, aufsteigend nach „ab“",
        Scope::Grade,
    );
    const GROUPS: ListDef = ("gruppen", "die Gruppen des Schemas", Scope::Group);
    match scope {
        Scope::RootStudent => &[GRADES, GROUPS],
        Scope::RootClass => &[
            GRADES,
            (
                "schueler",
                "die Schüler der Klasse, alphabetisch",
                Scope::Student,
            ),
        ],
        Scope::Student => &[GROUPS],
        Scope::Group => &[("aufgaben", "die Punkte der Gruppe", Scope::Item)],
        Scope::Item | Scope::Grade => &[],
    }
}

fn root_scope(kind: Kind) -> Scope {
    match kind {
        Kind::Student => Scope::RootStudent,
        Kind::Class => Scope::RootClass,
    }
}

/// Erzeugt `placeholder.md` aus denselben Tabellen, gegen die geprüft wird —
/// so kann die Beschreibung nicht von der Prüfung abweichen.
pub fn placeholder_doc() -> String {
    let mut out = String::from(
        "<!-- Diese Datei wird bei jedem Start der App neu geschrieben. Änderungen gehen verloren. -->\n\n\
         # Platzhalter für Druckvorlagen\n\n\
         Druckvorlagen sind Markdown-Dateien in diesem Ordner.\n\n\
         ## Dateinamen\n\n\
         * `print_test_student_NAME.md` — wird **je Schüler** ausgefüllt, jeder Schüler beginnt auf einer neuen Seite.\n\
         * `print_test_class_NAME.md` — wird **einmal je Klasse** ausgefüllt.\n\
         * `NAME` = `standard` sind die Vorlagen der App. Sie werden nicht gelesen; die App benutzt ihre eingebaute Fassung. Die Datei ist eine Kopie zum Abschreiben: unter anderem Namen speichern, z. B. `print_test_student_schulkopf.md`, und dort ändern.\n\
         * Eine Vorlage mit Fehler bricht den Druck ab; die Meldung nennt Datei und Zeile.\n\n\
         ## Einsetzen und Wiederholen\n\n\
         * `{{name}}` setzt einen Wert ein. Der Wert ist Text, nie Markdown.\n\
         * `{{#liste}}` … `{{/liste}}` wiederholt alles dazwischen je Eintrag — auch Tabellenzeilen. Steht die Markierung allein in einer Zeile, verschwindet die Zeile.\n\
         * `{{#wenn name}}` … `{{#sonst}}` … `{{/wenn}}` nimmt den ersten Teil, wenn der Wert nicht leer ist (bei einer Liste: wenn sie Einträge hat). `{{#sonst}}` ist freiwillig.\n\
         * Innerhalb einer Liste gilt zuerst der Wert des Eintrags, dann der der äußeren Ebene.\n\n",
    );
    for (title, scope) in [
        (
            "In Schüler-Vorlagen (`print_test_student_*`)",
            Scope::RootStudent,
        ),
        (
            "In Klassen-Vorlagen (`print_test_class_*`)",
            Scope::RootClass,
        ),
    ] {
        out.push_str(&format!("## {title}\n\n"));
        write_scope(&mut out, scope, 0);
    }
    out.push_str(
        "## Bilder\n\n\
         `![Logo](logo.png)` setzt ein Bild; die Datei liegt in diesem Ordner (PNG oder JPEG, auch in einem Unterordner, aber nicht außerhalb). \
         Das Bild steht allein in einem Absatz. Die Breite geben ein Titel an — `![Logo](logo.png \"40mm\")` — oder `img { width: 40mm; }` im Stil.\n\n\
         ## Markdown\n\n\
         Überschriften `#` bis `###`, Absätze, `**fett**`, `*kursiv*`, Listen (`-` oder `1.`), Tabellen mit `|`, `---` als Trennlinie, \
         `<!-- seitenumbruch -->` für eine neue Seite. Die Striche in der Trennzeile einer Tabelle bestimmen die Breite der Spalten (`|---|------|` = 1 zu 2), \
         Doppelpunkte die Ausrichtung (`:---:` mittig, `---:` rechts). Zwei Leerzeichen am Zeilenende oder `<br>` brechen die Zeile um. \
         Schrift ist immer Times; Zeichen außerhalb von Westeuropa erscheinen als „?“.\n\n\
         ## Stil (CSS)\n\n\
         Ein Block `<style> … </style>` irgendwo in der Vorlage ändert das Aussehen. Erlaubt:\n\n\
         * Selektoren: `body`, `h1`, `h2`, `h3`, `p`, `li`, `table`, `th`, `td`, `img`, `hr` (mit Komma mehrere). `body` gilt für alles und legt mit `margin` den Seitenrand fest.\n\
         * Eigenschaften: `font-size`, `font-weight` (bold/normal), `font-style` (italic/normal), `color`, `background`, `text-align` (left/center/right), `line-height`, `margin`, `margin-top`, `margin-bottom`, `padding`, `border` (`none` oder `0.5pt solid black`), `width`, `height`.\n\
         * Längen: `pt`, `px`, `mm`, `cm`, `in`. Farben: Namen wie `black`, `gray`, `lightgray`, `red`, `blue`, außerdem `#rrggbb` und `rgb(0, 0, 0)`.\n\
         * Etwas anderes — ein anderer Selektor, eine unbekannte Eigenschaft — bricht den Druck mit einer Meldung ab.\n",
    );
    out
}

fn write_scope(out: &mut String, scope: Scope, depth: usize) {
    let pad = "  ".repeat(depth);
    for fields in scalars(scope) {
        for (name, what) in fields {
            out.push_str(&format!("{pad}* `{{{{{name}}}}}` — {what}\n"));
        }
    }
    for (name, what, child) in lists(scope) {
        out.push_str(&format!(
            "{pad}* `{{{{#{name}}}}} … {{{{/{name}}}}}` — {what}; darin:\n"
        ));
        write_scope(out, *child, depth + 1);
    }
    if depth == 0 {
        out.push('\n');
    }
}

// --- Platzhalter: lesen und prüfen --------------------------------------------

#[derive(Debug, Clone)]
enum Node {
    Text(String),
    Var {
        name: String,
        line: usize,
    },
    Each {
        name: String,
        line: usize,
        body: Vec<Node>,
    },
    If {
        name: String,
        line: usize,
        then: Vec<Node>,
        otherwise: Vec<Node>,
    },
}

/// Eine Vorlage, deren Platzhalter gelesen und geprüft sind.
#[derive(Debug, Clone)]
pub struct Compiled {
    nodes: Vec<Node>,
    kind: Kind,
    /// Der Inhalt aller `<style>`-Blöcke samt Zeile, in der er beginnt.
    pub styles: Vec<(String, usize)>,
}

struct Frame {
    name: String,
    line: usize,
    is_if: bool,
    in_else: bool,
    then: Vec<Node>,
    otherwise: Vec<Node>,
}

/// Liest die Platzhalter und prüft sie gegen das, was es gibt. Der Fehler nennt
/// Datei und Zeile.
pub fn compile(template: &Template) -> Result<Compiled> {
    let file = &template.file;
    let fail = |line: usize, msg: String| template_error(format!("{file}, Zeile {line}: {msg}"));
    let (body, styles) = split_styles(&template.source).map_err(|(l, m)| fail(l, m))?;

    let mut root: Vec<Node> = Vec::new();
    let mut stack: Vec<Frame> = Vec::new();
    let mut line = 1usize;
    let mut rest = body.as_str();

    macro_rules! out {
        () => {
            match stack.last_mut() {
                None => &mut root,
                Some(f) if f.in_else => &mut f.otherwise,
                Some(f) => &mut f.then,
            }
        };
    }

    while let Some(start) = rest.find("{{") {
        let (text, after) = rest.split_at(start);
        let Some(end) = after.find("}}") else {
            return Err(fail(
                line + text.matches('\n').count(),
                "„{{“ ohne schließendes „}}“".into(),
            ));
        };
        let tag = after[2..end].trim().to_string();
        let tag_line = line + text.matches('\n').count();
        let mut text = text.to_string();
        let mut tail = &after[end + 2..];

        let is_block = tag.starts_with('#') || tag.starts_with('/');
        if is_block {
            // Steht die Markierung allein in ihrer Zeile, geht die ganze Zeile.
            let line_start = text.rfind('\n').map_or(0, |i| i + 1);
            let alone_before = text[line_start..].trim().is_empty();
            let after_blank = tail
                .find('\n')
                .map_or(tail.trim().is_empty(), |i| tail[..i].trim().is_empty());
            if alone_before && after_blank {
                text.truncate(line_start);
                tail = match tail.find('\n') {
                    Some(i) => &tail[i + 1..],
                    None => "",
                };
            }
        }
        if !text.is_empty() {
            out!().push(Node::Text(text));
        }

        if let Some(name) = tag.strip_prefix("#wenn ") {
            stack.push(Frame {
                name: name.trim().to_string(),
                line: tag_line,
                is_if: true,
                in_else: false,
                then: Vec::new(),
                otherwise: Vec::new(),
            });
        } else if tag == "#sonst" {
            match stack.last_mut() {
                Some(f) if f.is_if && !f.in_else => f.in_else = true,
                _ => {
                    return Err(fail(
                        tag_line,
                        "{{#sonst}} steht außerhalb von {{#wenn …}} oder zweimal".into(),
                    ));
                }
            }
        } else if let Some(name) = tag.strip_prefix('#') {
            stack.push(Frame {
                name: name.trim().to_string(),
                line: tag_line,
                is_if: false,
                in_else: false,
                then: Vec::new(),
                otherwise: Vec::new(),
            });
        } else if let Some(name) = tag.strip_prefix('/') {
            let name = name.trim();
            let Some(f) = stack.pop() else {
                return Err(fail(tag_line, format!("{{{{/{name}}}}} ohne Anfang")));
            };
            let expect = if f.is_if { "wenn" } else { f.name.as_str() };
            if name != expect {
                return Err(fail(
                    tag_line,
                    format!(
                        "{{{{/{name}}}}} schließt nicht {{{{#{}}}}} aus Zeile {} — erwartet wird {{{{/{expect}}}}}",
                        if f.is_if {
                            format!("wenn {}", f.name)
                        } else {
                            f.name.clone()
                        },
                        f.line
                    ),
                ));
            }
            let node = if f.is_if {
                Node::If {
                    name: f.name,
                    line: f.line,
                    then: f.then,
                    otherwise: f.otherwise,
                }
            } else {
                Node::Each {
                    name: f.name,
                    line: f.line,
                    body: f.then,
                }
            };
            out!().push(node);
        } else {
            if tag.is_empty() || tag.contains(char::is_whitespace) {
                return Err(fail(
                    tag_line,
                    format!("„{{{{{tag}}}}}“ ist kein Platzhalter"),
                ));
            }
            out!().push(Node::Var {
                name: tag,
                line: tag_line,
            });
        }
        // `tail` ist das Ende von `rest`: alles davor ist verbraucht.
        let consumed = rest.len() - tail.len();
        line += rest[..consumed].matches('\n').count();
        rest = tail;
    }
    if !rest.is_empty() {
        out!().push(Node::Text(rest.to_string()));
    }
    if let Some(f) = stack.pop() {
        return Err(fail(
            f.line,
            format!(
                "{{{{#{}}}}} wird nie geschlossen — es fehlt {{{{/{}}}}}",
                if f.is_if {
                    format!("wenn {}", f.name)
                } else {
                    f.name.clone()
                },
                if f.is_if { "wenn" } else { f.name.as_str() }
            ),
        ));
    }

    let compiled = Compiled {
        nodes: root,
        kind: template.kind,
        styles,
    };
    check(&compiled.nodes, &[root_scope(template.kind)], file)?;
    Ok(compiled)
}

/// Trennt `<style>`-Blöcke ab. Der Rest behält seine Zeilenzahl.
#[allow(clippy::type_complexity)]
fn split_styles(
    source: &str,
) -> std::result::Result<(String, Vec<(String, usize)>), (usize, String)> {
    let mut body = String::new();
    let mut styles = Vec::new();
    let mut rest = source;
    let mut line = 1usize;
    while let Some(start) = rest.find("<style>") {
        body.push_str(&rest[..start]);
        line += rest[..start].matches('\n').count();
        let Some(end) = rest[start..].find("</style>") else {
            return Err((line, "<style> wird nie mit </style> geschlossen".into()));
        };
        let css = &rest[start + 7..start + end];
        styles.push((css.to_string(), line));
        let lines = css.matches('\n').count();
        body.push_str(&"\n".repeat(lines));
        line += lines;
        rest = &rest[start + end + 8..];
    }
    body.push_str(rest);
    Ok((body, styles))
}

/// Prüft jeden Platzhalter gegen die Ebenen, in denen er steht.
fn check(nodes: &[Node], stack: &[Scope], file: &str) -> Result<()> {
    let fail = |line: usize, msg: String| template_error(format!("{file}, Zeile {line}: {msg}"));
    let has_scalar = |name: &str| {
        stack.iter().any(|s| {
            scalars(*s)
                .iter()
                .any(|f| f.iter().any(|(n, _)| *n == name))
        })
    };
    let find_list = |name: &str| {
        stack
            .iter()
            .rev()
            .find_map(|s| lists(*s).iter().find(|(n, _, _)| *n == name))
    };
    let known = || {
        let mut names: Vec<String> = Vec::new();
        for s in stack.iter().rev() {
            for f in scalars(*s) {
                names.extend(f.iter().map(|(n, _)| format!("{{{{{n}}}}}")));
            }
            names.extend(lists(*s).iter().map(|(n, _, _)| format!("{{{{#{n}}}}}")));
        }
        names.join(", ")
    };
    for node in nodes {
        match node {
            Node::Text(_) => {}
            Node::Var { name, line } => {
                if !has_scalar(name) {
                    let hint = if find_list(name).is_some() {
                        format!(
                            " — „{name}“ ist eine Liste, sie wird mit {{{{#{name}}}}} … {{{{/{name}}}}} wiederholt"
                        )
                    } else {
                        String::new()
                    };
                    return Err(fail(
                        *line,
                        format!(
                            "unbekannter Platzhalter {{{{{name}}}}}{hint}. Hier gibt es: {}",
                            known()
                        ),
                    ));
                }
            }
            Node::Each { name, line, body } => {
                let Some((_, _, child)) = find_list(name) else {
                    return Err(fail(
                        *line,
                        format!(
                            "unbekannte Liste {{{{#{name}}}}}. Hier gibt es: {}",
                            known()
                        ),
                    ));
                };
                let mut inner = stack.to_vec();
                inner.push(*child);
                check(body, &inner, file)?;
            }
            Node::If {
                name,
                line,
                then,
                otherwise,
            } => {
                if !has_scalar(name) && find_list(name).is_none() {
                    return Err(fail(
                        *line,
                        format!(
                            "unbekannter Platzhalter {{{{#wenn {name}}}}}. Hier gibt es: {}",
                            known()
                        ),
                    ));
                }
                check(then, stack, file)?;
                check(otherwise, stack, file)?;
            }
        }
    }
    Ok(())
}

// --- Platzhalter: ausfüllen ------------------------------------------------------

/// Die Werte einer Ebene.
#[derive(Debug, Default, Clone)]
pub struct Values {
    vars: HashMap<&'static str, String>,
    lists: HashMap<&'static str, Vec<Values>>,
}

impl Values {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn set(&mut self, name: &'static str, value: impl Into<String>) -> &mut Self {
        self.vars.insert(name, value.into());
        self
    }

    pub fn set_list(&mut self, name: &'static str, items: Vec<Values>) -> &mut Self {
        self.lists.insert(name, items);
        self
    }

    /// Fügt die Werte von `other` hinzu (für die oberste Ebene einer
    /// Schüler-Vorlage: allgemeine Werte plus ein Schüler).
    pub fn extend(&mut self, other: Values) -> &mut Self {
        self.vars.extend(other.vars);
        self.lists.extend(other.lists);
        self
    }
}

impl Compiled {
    pub fn kind(&self) -> Kind {
        self.kind
    }

    /// Setzt die Werte ein. Das Ergebnis ist Markdown.
    pub fn render(&self, root: &Values) -> String {
        let mut out = String::new();
        render_nodes(&self.nodes, &[root], &mut out);
        out
    }
}

fn lookup_var<'a>(stack: &[&'a Values], name: &str) -> Option<&'a String> {
    stack.iter().rev().find_map(|v| v.vars.get(name))
}

fn lookup_list<'a>(stack: &[&'a Values], name: &str) -> Option<&'a Vec<Values>> {
    stack.iter().rev().find_map(|v| v.lists.get(name))
}

fn render_nodes(nodes: &[Node], stack: &[&Values], out: &mut String) {
    for node in nodes {
        match node {
            Node::Text(t) => out.push_str(t),
            Node::Var { name, .. } => {
                if let Some(v) = lookup_var(stack, name) {
                    out.push_str(&escape(v));
                }
            }
            Node::Each { name, body, .. } => {
                for item in lookup_list(stack, name).into_iter().flatten() {
                    let mut inner = stack.to_vec();
                    inner.push(item);
                    render_nodes(body, &inner, out);
                }
            }
            Node::If {
                name,
                then,
                otherwise,
                ..
            } => {
                let yes = lookup_var(stack, name).map_or_else(
                    || lookup_list(stack, name).is_some_and(|l| !l.is_empty()),
                    |v| !v.trim().is_empty(),
                );
                render_nodes(if yes { then } else { otherwise }, stack, out);
            }
        }
    }
}

/// Macht aus einem Wert Text, den Markdown nicht deutet: Satzzeichen mit
/// Backslash, Zeilenumbrüche als harter Umbruch.
pub fn escape(value: &str) -> String {
    let mut out = String::with_capacity(value.len());
    for c in value.chars() {
        match c {
            '\\' | '*' | '_' | '|' | '[' | ']' | '!' | '`' | '<' | '>' | '#' | '-' | '+' | '.' => {
                out.push('\\');
                out.push(c);
            }
            '\n' => out.push(crate::markdown::BREAK),
            '\r' => {}
            c => out.push(c),
        }
    }
    out
}

// --- Die Standardvorlagen --------------------------------------------------------

const STANDARD_NOTE: &str = "<!-- Das ist die Standardvorlage der App. Sie wird NICHT gelesen: Die App benutzt ihre eingebaute Fassung, damit der Ausdruck immer geht. Zum Anpassen diese Datei unter anderem Namen speichern (z. B. {NAME}schulkopf.md) und dort ändern. Alle Platzhalter stehen in placeholder.md. -->\n\n";

/// Der Text der Standardvorlage — genau das, was die App ohne eigene Vorlage
/// druckt.
pub fn standard_source(kind: Kind) -> String {
    let (prefix, body) = match kind {
        Kind::Student => (STUDENT_PREFIX, STUDENT_STANDARD),
        Kind::Class => (CLASS_PREFIX, CLASS_STANDARD),
    };
    format!("{}{body}", STANDARD_NOTE.replace("{NAME}", prefix))
}

const STUDENT_STANDARD: &str = r#"<style>
body { font-size: 10pt; margin: 50pt; }
h1 { font-size: 16pt; }
h2 { font-size: 12pt; }
th { background: lightgray; }
</style>

# {{pruefung}}

Schüler/in: {{schueler}}

Klasse: {{klasse}}

{{#wenn begruendungen}}
| Aufgabe | Beschreibung | Punkte | Max. | Begründung |
|:--------|:-------------|:------:|:----:|:-----------|
{{#gruppen}}
| **{{name}}** | | **{{punkte}}** | **{{max_punkte}}** | |
{{#aufgaben}}
| {{name}} | {{beschreibung}} | {{punkte}} | {{max_punkte}} | {{begruendung}} |
{{/aufgaben}}
{{/gruppen}}
| **Gesamtpunkte** | | **{{gesamtpunkte}}** | **{{max_punkte}}** | |
| **Note** | | **{{note}}** | | |
{{#sonst}}
| Aufgabe | Beschreibung | Punkte | Max. |
|:--------|:-------------|:------:|:----:|
{{#gruppen}}
| **{{name}}** | | **{{punkte}}** | **{{max_punkte}}** |
{{#aufgaben}}
| {{name}} | {{beschreibung}} | {{punkte}} | {{max_punkte}} |
{{/aufgaben}}
{{/gruppen}}
| **Gesamtpunkte** | | **{{gesamtpunkte}}** | **{{max_punkte}}** |
| **Note** | | **{{note}}** | |
{{/wenn}}

{{#wenn abschluss_satz}}
## Abschluss

{{abschluss_satz}}
{{/wenn}}

## Bewertungsmatrix

| Note | Punkte (ab – bis) |
|:----:|:-----------------:|
{{#noten}}
| {{name}} | {{von}} – {{bis}} |
{{/noten}}
"#;

const CLASS_STANDARD: &str = r#"<style>
body { font-size: 10pt; margin: 50pt; }
h1 { font-size: 16pt; }
th { background: lightgray; }
</style>

# {{pruefung}}

Klasse: {{klasse}} — {{anzahl_schueler}} Schüler/innen, Höchstpunktzahl {{max_punkte}}

## Ergebnisse

| Schüler/in | Punkte | Note |
|:-----------|:------:|:----:|
{{#schueler}}
| {{schueler}} | {{gesamtpunkte}} | {{note}} |
{{/schueler}}

## Notenspiegel

| Note | Punkte (ab – bis) | Anzahl |
|:----:|:-----------------:|:------:|
{{#noten}}
| {{name}} | {{von}} – {{bis}} | {{anzahl}} |
{{/noten}}
"#;

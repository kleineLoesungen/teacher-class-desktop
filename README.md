# Teacher Class

Eine schlichte Desktop-App, die Lehrkräften die **Korrektur von Prüfungen**
erleichtert. Läuft auf **macOS, Linux und Windows**.

Man pflegt Klassen und Schüler, legt eine Prüfung mit **Prüfungsschema** und
**Bewertungsmatrix** an und exportiert sie als **Excel-Datei**. Darin trägt man
die Punkte je Schüler ein; Summen und Noten rechnen **Formeln**. Ändert man die
Bewertungsmatrix, ändern sich die Noten. Bewertet werden kann auch **in der
App** (Punkte und, wenn gewünscht, eine Begründung je Prüfkriterium); der Export
enthält dann die Punkte. Zum Ausdrucken gibt es pro Klasse ein **PDF mit einer
Seite je Schüler** (mit Begründungen, wenn die Prüfung welche hat).

Technisch: **Tauri 2**, Oberfläche in **React und TypeScript**, Fachlogik in
**Rust** mit eingebetteter **SQLite**. Dieselbe Fachlogik bedient auch eine
Kommandozeile (`teacher-class`).

---

## Was die App kann

### Klassen und Schüler

* Eine **Klasse** hat einen Namen aus Buchstaben und Zahlen (`7a`) und ein
  vierstelliges Jahr. Angezeigt wird sie als `7a (2025)`.
* **Schüler** heißen `Nachname, Vorname`. Mehrere legt man auf einmal an: ein
  Textfeld, eine Zeile je Schüler. Gespeichert wird alles oder nichts; ein
  Fehler nennt die Zeilennummer.
* Einen Klassenwechsel gibt es nicht, einen Import aus Dateien noch nicht.
* **Archiv:** Klassen und Prüfungen lassen sich **archivieren**. Sie
  verschwinden aus der Liste in den eingeklappten Bereich „Archiv“ und lassen
  sich von dort zurückholen; nichts geht verloren. Archivierte Klassen werden
  bei neuen Prüfungen nicht mehr zur Wahl angeboten. In der Kommandozeile:
  `class archive|unarchive`, `exam archive|unarchive`.

### Prüfungen

Eine Prüfung gilt für eine oder mehrere Klassen und besteht aus:

* **Prüfungsschema:** Gruppen mit Namen; darin **Punkte** (Aufgaben) mit Namen
  und einer Höchstpunktzahl. Ganze und halbe Punkte sind erlaubt (`2,5`). Die
  Höchstpunktzahl einer Gruppe und der ganzen Prüfung wird berechnet. Zu jedem
  Punkt (Prüfkriterium) kann man eine **Beschreibung** hinterlegen — optional.
* **Begründungen:** Beim Anlegen wählt man, ob zu jedem Prüfkriterium eine
  Begründung gepflegt wird. Das ändert die Form des Exports (siehe unten).
* **Bewertungsmatrix:** Noten frei benennbar, üblich `1` bis `6`. Je Note trägt
  man nur das **„ab“** ein (Punkte). Das **„bis“** wird berechnet — eine halbe
  Punktstufe unter dem „ab“ der nächsten Note. Dadurch gibt es weder Lücken
  noch Überschneidungen. Die niedrigste Note beginnt bei 0.

### Bewerten in der App

In der Prüfung öffnet **„Bewerten“** die Bewertung einer Klasse: Schüler
wählen, je Prüfkriterium die erreichten Punkte eintragen (ganz oder halb, `2,5`)
und — wenn die Prüfung Begründungen vorsieht — eine Begründung oder einen
Hinweis. Gespeichert wird, sobald ein Feld verlassen wird. Unter den Kriterien steht ein
**Abschluss-Satz** („Tolle Leistung! Weiter so“) neben der Note, in jeder
Prüfung, auch ohne Begründungen. Beim Tippen in einer Begründung oder im
Abschluss-Satz erscheinen **Vorschläge**: Texte anderer Schüler derselben
Prüfung, die alle getippten Wörter enthalten, die häufigsten zuerst; ein Klick
übernimmt den Text. Der Abschluss-Satz steht im PDF-Ausdruck, nicht im
Excel-Export. In der Kommandozeile: `exam remark`, `exam suggest`. Gruppensummen,
Gesamtpunkte und die Note berechnet die Fachlogik; „nicht eingetragen“ ist
etwas anderes als 0, und solange nichts eingetragen ist, gibt es keine Note.

Ändert man die Prüfung später, bleiben die Einträge, soweit sie noch passen
(gleiche Stelle im Schema): Fällt ein Punkt weg, verschwindet sein Eintrag;
sinkt seine Höchstpunktzahl unter die vergebenen Punkte, werden die Punkte
zurückgenommen, die Begründung bleibt. Schaltet man die Begründungen ab,
entfallen sie.

### Excel-Export

**„Als Excel exportieren“** in der Prüfung öffnet einen Speichern-Dialog.
Die `.xlsx`-Datei enthält alles, was in der App eingetragen ist, und lässt sich
in Excel weiter bearbeiten:

| Blatt | Inhalt |
|---|---|
| **Bewertung** | Prüfungsschema (mit Beschreibungen) und Bewertungsmatrix. Hier ändert man „ab“ und Höchstpunkte; alles andere folgt per Formel. |
| **je Klasse** | Blattname `7a (2025)`: eine Matrix, bei Prüfungen mit Begründungen Kacheln, siehe unten. |

Das Klassenblatt ist bei Prüfungen **ohne Begründungen** eine **Matrix**:
Schüler in den Zeilen, Punkte des Schemas in den Spalten, Summe je Gruppe,
Gesamtpunkte, zuletzt die **Note** aus der Bewertungsmatrix. Solange nichts
eingetragen ist, bleibt die Note leer. Einen **Abschluss-Satz** exportiert die
App, sobald einer eingetragen ist, als Text hinter der Note.

Bei Prüfungen **mit Begründungen** ist das Klassenblatt stattdessen in **Kacheln**
gegliedert: je Schüler ein Block mit Aufgabe, Punkten, Höchstpunkten und
Begründung, Summe je Gruppe, Gesamtpunkten, Note (Formeln wie oben) und dem
Abschluss-Satz; nach jedem Block beginnt eine neue Druckseite. Notenschlüssel und
Schnitt gibt es dann im Excel nicht.

**Notenschlüssel** (nur bei Prüfungen ohne Begründungen): Auf dem Blatt „Bewertung“ steht neben der Bewertungsmatrix je
Klasse eine Spalte **Anzahl** (wie oft jede Note vorkam) und darunter der
**Schnitt** auf zwei Nachkommastellen — beides Formeln auf die Noten des
Klassenblatts. In der App zeigt die Bewertung nach Wahl der Klasse einen
einklappbaren **Notenschlüssel** als Tabelle (Spalten Note und Anzahl, darunter
„Schnitt 2,60“). Die Klasse wählt man über die Knöpfe über der Schülerliste.
Unter der **Bewertungsmatrix** der Prüfung steht dezent die Notenverteilung je
Klasse mit Schnitt („7a (2025): 1 × 3 · 3 × 5 · Schnitt 2,60“). Sie rechnet
schon mit den noch nicht gespeicherten „ab“-Werten und zieht beim Tippen nach —
so sieht man sofort, was eine geänderte Grenze für die Noten bedeutet.

### PDF-Ausdruck

**„Als PDF drucken …“** in der Bewertung öffnet einen Dialog: oben die **Klasse**
(vorgewählt ist die offene), darunter die **Druckvorlage** (je Schüler oder je
Klasse). „PDF speichern …“ öffnet den Speichern-Dialog und schreibt **ein PDF für
diese Klasse**, A4
hochkant. Die **Standardvorlage „je Schüler“** setzt **eine Seite je
Schüler** — Aufgabe, Beschreibung, Punkte, Höchstpunkte und (bei Prüfungen mit
Begründungen) die Begründung, je Gruppe eine Summenzeile, darunter Gesamtpunkte,
Note, Abschluss-Satz und die Bewertungsmatrix. Die Standardvorlage „je Klasse“
ist eine Übersicht mit allen Schülern, Punkten und Noten sowie dem Notenspiegel.
Die Seitenaufteilung berechnet die App selbst (`crates/core/src/export_pdf.rs`);
ein Tabellenprogramm ist nicht beteiligt. Ist eine Tabelle durch lange
Begründungen höher als eine Seite, läuft sie auf der nächsten weiter, mit
wiederholtem Tabellenkopf; der nächste Schüler beginnt immer neu.

#### Eigene Druckvorlagen

Das Aussehen steuern **Markdown-Dateien mit Platzhaltern** im Ordner `Templates`
**neben der Datenbank** (die Bewertung zeigt den Pfad; auf der Kommandozeile
`teacher-class exam templates`). Wechselt man den Speicherort der Datenbank,
gelten die Vorlagen dort; fehlen sie, legt die App die Standards an.

* Der **Dateiname** entscheidet: `print_test_student_NAME.md` wird für jeden
  Schüler ausgefüllt, `print_test_class_NAME.md` einmal für die Klasse. Eigene
  Vorlagen bekommen einen eigenen `NAME`, z. B. `print_test_student_schulkopf.md`.
* `print_test_student_standard.md` und `print_test_class_standard.md` sind die
  **Vorlagen der App** — als Kopie zum Abschreiben angelegt, aber **nie
  gelesen**: Die App benutzt ihre eingebaute Fassung, damit der Ausdruck immer
  geht. Zum Anpassen die Datei unter anderem Namen speichern.
* `placeholder.md` im selben Ordner listet alle Platzhalter (`{{schueler}}`,
  `{{gesamtpunkte}}`, Wiederholungen wie `{{#gruppen}} … {{/gruppen}}`,
  Auswahl mit `{{#wenn begruendungen}}`). Sie wird bei jedem Start neu
  geschrieben.
* **Markdown:** Überschriften, Absätze, **fett**, *kursiv*, Listen, Tabellen,
  Trennlinie, `<!-- seitenumbruch -->` und **Bilder** (`![Logo](logo.png)`, PNG
  oder JPEG aus dem Vorlagenordner). **Stil:** ein Block `<style> … </style>`
  mit einer kleinen CSS-Teilmenge (Schriftgröße, Farbe, Ausrichtung, Ränder,
  Rahmen, Bildbreite); was die App nicht kennt, ist ein Fehler.
* **Fehler in einer Vorlage brechen den Druck ab**, mit Datei, Zeile und Grund
  (in der Oberfläche oben in der Leiste). Einen Editor gibt es nicht — man
  bearbeitet die Datei in einem Textprogramm. Werte aus der Datenbank sind
  immer Text, nie Markdown: Ein `*` im Namen macht nichts fett.

Die **Schrift** ist Times (die in jedem PDF-Programm eingebauten Times-Schriften,
sie werden nicht eingebettet — es liegt keine Schriftdatei im Projekt). Zeichen
außerhalb von Westeuropa (z. B. polnisches Ł) erscheinen als „?“.

Der Export ist einseitig: Änderungen in Excel gelangen nicht zurück in die App.

### Kommandozeile

```bash
teacher-class class add 7a 2025
teacher-class student add <KLASSE> "Muster, Max
Beispiel, Eva"
teacher-class exam add "Klassenarbeit 1" --class <KLASSE> \
    --group "Aufgabe 1: a=4; b=2,5" --group "Aufgabe 2: a=13,5" \
    --grade 6=0 --grade 3=10 --grade 1=17
teacher-class exam show <PRÜFUNG>
teacher-class exam score <PRÜFUNG> <SCHÜLER> 1 1 --points 2,5
teacher-class exam results <PRÜFUNG> --class <KLASSE>
teacher-class exam key <PRÜFUNG> --class <KLASSE>
teacher-class exam export <PRÜFUNG> --out Klassenarbeit.xlsx
```

Jeder Befehl kann `--json`. Alles dazu in [docs/CLI.md](docs/CLI.md).
Oberfläche und Kommandozeile arbeiten auf **derselben** Datenbank; wo sie liegt,
sagt `teacher-class paths`. Den Speicherort kann man wählen — in den
Einstellungen der Oberfläche oder mit `teacher-class db use|create|reset`. Fehlt
die Datenbank am gewählten Ort, fragt die App, ob eine neue angelegt oder ein
anderer Ort gewählt werden soll.

---

## Entwickeln

Voraussetzungen auf **allen** Systemen: [rustup](https://rustup.rs),
[Node.js 22](https://nodejs.org), [just](https://github.com/casey/just).

| | just installieren | zusätzlich |
|---|---|---|
| macOS | `brew install just` | Xcode-Kommandozeilenwerkzeuge |
| Linux | `cargo install just` | die WebKitGTK-Pakete unten |
| Windows | `winget install Casey.Just` | [Visual Studio Build Tools](https://visualstudio.microsoft.com/visual-cpp-build-tools/) mit C++, [WebView2](https://developer.microsoft.com/microsoft-edge/webview2/) (auf Windows 11 schon da) |

```bash
# Linux
sudo apt install libwebkit2gtk-4.1-dev libxdo-dev libssl-dev \
    libayatana-appindicator3-dev librsvg2-dev build-essential
```

(Fedora, Arch und andere: [Tauri-Voraussetzungen](https://v2.tauri.app/start/prerequisites/).)

**Kein Git Bash, keine Shell-Skripte.** Jedes `just`-Rezept ist ein schlichter
Befehl; alles Weitere liegt in `scripts/*.mjs` und läuft über Node.

```bash
just setup          # npm-Pakete, Werkzeuge, einmal durchbauen
just dev            # Desktop-Anwendung starten, lädt bei Änderungen neu
just dev-sandbox    # dasselbe mit leerer Wegwerf-Datenbank
just cli class list # die Kommandozeile
just check          # alles prüfen, vor jedem Commit
```

Ohne die echten Daten anzufassen: `just sandbox class list`.

### Wo was liegt

```
crates/
├── core/                    ← FACHLOGIK. Kennt weder clap noch tauri noch stdout.
│   ├── migrations/            Nummerierte SQL-Dateien, ins Binary einkompiliert
│   ├── src/class.rs           Klassen
│   ├── src/student.rs         Schüler einer Klasse, Massen-Eintrag als Text
│   ├── src/exam.rs            Prüfungen: Schema, Bewertungsmatrix, Prüfung
│   ├── src/export.rs          Excel-Export (rust_xlsxwriter)
│   ├── src/export_pdf.rs      PDF-Ausdruck: Seitenaufteilung und PDF-Datei
│   ├── src/template.rs        Druckvorlagen: Ordner, Platzhalter, Standardvorlagen
│   ├── src/markdown.rs        Markdown-Teilmenge der Vorlagen
│   ├── src/style.rs           CSS-Teilmenge der Vorlagen
│   ├── src/image.rs           Logo (PNG, JPEG) fürs PDF
│   ├── src/config.rs          Einstellungen, TOML
│   ├── src/paths.rs           Die einzige Stelle mit macOS/Linux-Unterschieden
│   └── tests/layering.rs      Der Wächter über die Schichtregel (Rust + TypeScript)
│
├── cli/                     ← Kommandozeile `teacher-class`. Dünn.
│
└── desktop/                 ← Tauri-Schale. Dünn.
    ├── tauri.conf.json        Fenster, CSP, Bündel
    ├── capabilities/          Was das Fenster darf (u. a. Speichern-Dialog)
    ├── src/commands.rs        Die IPC-Befehle
    ├── src/error.rs           ApiError — die Fehlerhülle
    ├── tests/ipc_contract.rs  Befehle ohne Fenster aufrufen; Rust ↔ api.ts
    └── tests/bindings.rs      Erzeugt ui/src/bindings.ts

ui/
├── index.html
└── src/
    ├── api.ts                 ← Die EINZIGE Stelle, die mit Rust spricht
    ├── bindings.ts            Erzeugt aus Rust — nicht von Hand ändern
    ├── App.tsx                Zustand, ruft api
    ├── components/            Zeichnen nur: Klassen, Schüler, Prüfungen
    ├── lib/                   Reine Hilfen: Prüfungsentwurf, Format, Farbschema
    ├── theme.css              Die EINZIGE Datei mit Farben
    └── app.css                Aufbau und Abstände

scripts/
├── package-cli.mjs          dist/teacher-class-cli-<version>-<os>-<arch>.tar.gz
└── tasks.mjs                Was im Justfile keine schlichte Zeile wäre

.claude/agents/              Architektur · Dokumentation · Sicherheit
.github/workflows/           Bau auf drei Systemen, Issues, PR-Durchsicht
```

Regeln, Festgenageltes und die Tauri-2-Fallstricke stehen in
[CLAUDE.md](CLAUDE.md). Die wichtigste: **Prüfung gehört nach `core`.** Was für
die Oberfläche gilt, gilt so auch für die Kommandozeile.

---

## Vom Telefon aus arbeiten

Issue schreiben, `@claude` hineinschreiben, warten. Claude setzt um, lässt
`just check` laufen und öffnet einen PR; drei Prüf-Agents sehen ihn durch und
schreiben je einen Kommentar; die CI baut auf allen drei Systemen.

Dafür braucht das Repository einmalig das Secret `CLAUDE_CODE_OAUTH_TOKEN`
(die Einrichtung ist abgeschlossen). Auslösen kann das **nur der
Repo-Inhaber**. Sperren und Kosten: [docs/AUTOMATION.md](docs/AUTOMATION.md).

---

## Ausliefern

```bash
just bundle                   # für dieses System
just bundle-macos-universal   # macOS: Apple Silicon + Intel in einem Bündel
just package-cli              # die Kommandozeile als Tarball
```

| System | Ergebnis |
|---|---|
| macOS | `.app` + `.dmg` |
| Linux | `.deb` + `.AppImage` |
| Windows | `.msi` + `-setup.exe` |

Gebaut wird immer **auf** dem Zielsystem — ein `.msi` entsteht nur auf Windows.
Dafür gibt es die Bau-Workflows unter `.github/workflows/`. Ergebnisse unter
`target/release/bundle/` bzw. `dist/`. Signieren und Notarisieren:
[docs/RELEASE.md](docs/RELEASE.md).

> **Noch offen für den ersten Release:** `assets/icon.png` ist noch das
> Symbol der Vorlage. Ein eigenes 1024×1024-PNG dorthin legen und `just icons`
> ausführen (`icon.ico` entsteht nur auf Windows, `icon.icns` nur auf macOS).

---

## Weiterlesen

| Datei | Inhalt |
|---|---|
| [CLAUDE.md](CLAUDE.md) | Die Regeln. Tauri-2-Fallstricke, festgenagelte Versionen, Schichtregel |
| [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md) | Warum die Schichten so geschnitten sind, wie die IPC-Grenze und der Excel-Export aussehen |
| [docs/RECIPES.md](docs/RECIPES.md) | Neuer Befehl, neue Ressource, Excel-Blatt ändern, npm-Paket, Plugin |
| [docs/DESIGN.md](docs/DESIGN.md) | Aussehen ändern, Farbmarken, Symbole |
| [docs/CLI.md](docs/CLI.md) | Befehle, Rückgabewerte, JSON-Format |
| [docs/AUTOMATION.md](docs/AUTOMATION.md) | Entwicklung über Issues, die drei Prüf-Agents, Einrichtung |
| [docs/RELEASE.md](docs/RELEASE.md) | Bündeln, signieren, notarisieren, veröffentlichen |

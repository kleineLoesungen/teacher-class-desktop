# Teacher Class

Eine schlichte Desktop-App, die Lehrkräften die **Korrektur von Prüfungen**
erleichtert. Läuft auf **macOS, Linux und Windows**.

Man pflegt Klassen und Schüler, legt eine Prüfung mit **Prüfungsschema** und
**Bewertungsmatrix** an und exportiert sie als **Excel-Datei**. Darin trägt man
die Punkte je Schüler ein; Summen und Noten rechnen **Formeln**. Ändert man die
Bewertungsmatrix, ändern sich die Noten. Bewertet werden kann auch **in der
App** (Punkte und, wenn gewünscht, eine Begründung je Prüfkriterium); der Export
enthält dann alles. Mit Begründungen gibt es **eine Bewertungskachel je
Schüler**, eine Druckseite pro Schüler.

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
Hinweis. Gespeichert wird, sobald ein Feld verlassen wird. Gruppensummen,
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
| **je Klasse** | Blattname `7a (2025)`. Die Form hängt von den Begründungen ab, siehe unten. |

**Ohne Begründungen** ist das Klassenblatt eine **Matrix**: Schüler in den
Zeilen, Punkte des Schemas in den Spalten, Summe je Gruppe, Gesamtpunkte,
zuletzt die **Note** aus der Bewertungsmatrix. Solange nichts eingetragen ist,
bleibt die Note leer.

**Mit Begründungen** ist es **eine Bewertungskachel je Schüler**, untereinander
auf demselben Blatt: Aufgabe, Beschreibung, Punkte, Höchstpunkte und
Begründung, je Gruppe eine Summenzeile, darunter Gesamtpunkte, Note und die
Bewertungsmatrix. Vor jeder Kachel steht ein Seitenumbruch; das Blatt ist auf
A4 hochkant mit festem Druckmaßstab eingestellt. **Drucken:** In Excel das
Klassenblatt drucken oder als PDF speichern — jeder Schüler ist genau eine Seite
(einzelne Schüler über „Seiten von … bis …“). Der Druckmaßstab ist bewusst fest
und nicht „auf eine Seite anpassen“, weil Excel dabei die Seitenumbrüche
ignoriert. Sehr lange Beschreibungen oder Begründungen können eine Kachel über
eine Seite hinaus verlängern.

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

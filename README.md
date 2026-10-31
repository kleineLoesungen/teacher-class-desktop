# Teacher Class — Desktop-Anwendungen mit Tauri, Weboberfläche und Kommandozeile

Startpunkt für Desktop-Werkzeuge auf **macOS, Linux und Windows**, deren Oberfläche aus
dem Web kommt: **React und TypeScript** im Webview des Betriebssystems, damit
Browserpakete wie **mermaid.js**, marked oder Chart-Bibliotheken einfach über
npm benutzbar sind. Dahinter **Tauri 2** und **eine** Fachlogik in Rust, die
auch ein Kommandozeilenwerkzeug bedient. Daten in einer **eingebetteten
SQLite**.

Gemacht für die Arbeit mit KI-Assistenten: klare Regeln in
[CLAUDE.md](CLAUDE.md) (inklusive der Tauri-1-Fallen, in die Sprachmodelle
tappen), aus Rust erzeugte TypeScript-Typen, fertige Rezepte, ein strenges
`just check` — und Tests, die die Architektur und die IPC-Grenze bewachen.

Das Gegenstück ohne Webview, in reinem Rust mit egui:
[vibe-starter-desktop](https://github.com/kleineLoesungen/vibe-starter-desktop).

Die Einrichtung mit dem Claude OAuth Token ist abgeschlossen.

---

## In fünf Minuten laufen

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
Befehl; alles Weitere liegt in `scripts/*.mjs` und läuft über Node. Dieselben
Befehle auf allen drei Systemen.

```bash
just setup    # npm-Pakete, Werkzeuge, einmal durchbauen
just dev      # Desktop-Anwendung starten, lädt bei Änderungen neu
```

Im leeren Fenster **„Beispiel mit Diagramm anlegen"** klicken — die Notiz
enthält zwei Mermaid-Diagramme.

Und dieselbe Anwendung auf der Kommandozeile:

```bash
just cli note list
just cli note add "Einkauf" --body "Milch, Brot"
just cli note list --json | jq '.[].title'
```

Beides arbeitet auf **derselben** Datenbank. Wo die liegt, sagt `just cli paths`.
Ohne die echten Daten anzufassen: `just dev-sandbox` und `just sandbox note list`.

---

## Eigenes Projekt aus dem Kit

Das Kit selbst bleibt unverändert. Zwei Wege zu einer eigenen Kopie:

**Über GitHub** — der grüne Knopf **„Use this template"** legt ein eigenes
Repository mit frischer Historie an. Danach klonen und umbenennen:

```bash
git clone git@github.com:<du>/notizbuch.git
cd notizbuch
node scripts/new-project.mjs notizbuch "Notizbuch"
just setup && just check
```

**Lokal**, ohne Umweg über GitHub:

```bash
git clone https://github.com/kleineLoesungen/teacher-class notizbuch
cd notizbuch
node scripts/new-project.mjs notizbuch "Notizbuch"
git remote remove origin        # zeigt sonst noch auf das Kit
just setup && just check
```

`new-project.mjs` benennt um: Kistennamen, Binaries, Bündelkennung,
Umgebungsvariablen, Datenverzeichnis, Fenstertitel, `package.json`,
`tauri.conf.json`, Workflows.

**Das Skript direkt nach dem Klonen ausführen.** Bis dahin heißt das Projekt
noch „teacher-class" und benutzt dasselbe Datenverzeichnis wie das Kit — zwei
Projekte mit demselben Namen teilen sich unbemerkt eine Datenbank.

### Spätere Verbesserungen aus dem Kit holen

```bash
git remote add kit https://github.com/kleineLoesungen/teacher-class
git fetch kit
git log --oneline kit/main
```

Dann je nachdem, **was** du holst — und der Unterschied ist wichtiger, als er
aussieht:

**Infrastruktur — Workflows, `docs/`, `scripts/`, `deny.toml`, `biome.json`:**

```bash
git checkout kit/main -- .github/workflows docs/AUTOMATION.md
git commit -m "Workflows vom Kit übernehmen"
```

Diese Dateien enthalten **keine projektspezifischen Stellen** (geprüft: kein
Projektname in `.github/workflows/`). Es gibt also nichts zusammenzuführen —
die Fassung aus dem Kit ist die richtige, Punkt. `git checkout` nimmt sie am
Stück und kann nicht in einen Konflikt laufen.

**Code — alles unter `crates/` und `ui/src/`:**

```bash
git cherry-pick <sha>
```

Hier hast du eigene Änderungen, hier ist Zusammenführen richtig.

> **Warum die Unterscheidung:** Ein `cherry-pick` auf eine Infrastrukturdatei,
> die du einmal von Hand angefasst hast, endet im Konflikt. Bleiben die Marker
> versehentlich in einer Workflow-Datei stehen, ist das YAML ungültig — und
> GitHub führt den Workflow dann **stillschweigend gar nicht mehr aus**. Der
> Auslöser scheint kaputt, dabei ist es die Datei. Unter *Actions* erscheint
> dann ein roter Lauf namens `.github/workflows/…`; das ist das Erkennungs-
> zeichen.

Ein `merge` wäre in beiden Fällen der falsche Griff — er zöge die ganze
Kit-Geschichte herein.

---

## Vom Telefon aus arbeiten

Issue schreiben, `@claude` hineinschreiben, warten. Claude setzt um, lässt
`just check` laufen und öffnet einen PR; drei Prüf-Agents sehen ihn durch und
schreiben je einen Kommentar; die CI baut auf allen drei Systemen.

Dafür fehlt einmalig ein Repository-Secret `CLAUDE_CODE_OAUTH_TOKEN` —
Einrichtung, Sperren und Kosten stehen in
[docs/AUTOMATION.md](docs/AUTOMATION.md).

Auslösen kann das **nur der Repo-Inhaber**. Dieses Repository ist öffentlich;
ohne diese Sperre könnte jeder Fremde mit einem `@claude`-Kommentar Aufträge
in deiner CI ausführen lassen.

---

## Was drin ist

| Bereich | Umsetzung |
|---|---|
| Aufbau | Arbeitsbereich `core` · `cli` · `desktop` + `ui/`, bewacht von einem Test |
| Desktop | Tauri 2.12 auf **macOS, Linux und Windows** — Webview des Systems, Fenster in wenigen MB statt Chromium |
| Ein Codepfad | **Keine einzige `#[cfg(target_os)]`** im Rust-Code, kein Bash in der Werkzeugkette — beides von Tests bewacht |
| Oberfläche | React 19, TypeScript 7, Vite 8 — kein CSS-Framework, keine Zustandsbibliothek |
| Browserpakete | mermaid 12 (nachgeladen), marked, DOMPurify — alles über npm, läuft offline |
| IPC | Befehle als `async fn`, eine Fehlerhülle wie im CLI-JSON, TS-Typen aus Rust (ts-rs) |
| Sicherheit | CSP ohne fremde Quellen, Capabilities, bereinigtes Markdown, Verweise im Standardbrowser |
| Kommandozeile | clap 4, `--json` bei jedem Befehl, eigene Rückgabewerte, Handbuch, Vervollständigung |
| Daten | SQLite eingebettet, `STRICT`, WAL, nummerierte Migrationen |
| Einstellungen | TOML, von Oberfläche und CLI gemeinsam benutzt; hell/dunkel/System, Akzentfarbe |
| Prüfung | Feldgenaue Fehler aus Rust — in der Oberfläche am Feld, in der CLI als `error.fields` |
| Ausliefern | `.app` + `.dmg` (auch universal), `.deb` + `.AppImage`, `.msi` + `-setup.exe`, CLI-Tarball |
| Automatik | Bau auf drei Systemen, Entwicklung über Issues, drei Prüf-Agents auf jedem PR |
| Qualität | rustfmt, Clippy, Biome, `tsc`, `cargo deny`, `npm audit` — Warnungen sind Fehler |
| Tests | Fachlogik, Schichtwächter, CLI-Schnappschüsse, IPC ohne Fenster, Oberfläche mit `mockIPC` |

---

## Wo was liegt

```
crates/
├── core/                    ← FACHLOGIK. Kennt weder clap noch tauri noch stdout.
│   ├── migrations/            Nummerierte SQL-Dateien, ins Binary einkompiliert
│   ├── src/note.rs            Beispielressource zum Kopieren
│   ├── src/config.rs          Einstellungen, TOML
│   ├── src/paths.rs           Die einzige Stelle mit macOS/Linux-Unterschieden
│   └── tests/layering.rs      Der Wächter über die Schichtregel (Rust + TypeScript)
│
├── cli/                     ← Kommandozeile `teacher-class`. Dünn.
│
└── desktop/                 ← Tauri-Schale. Dünn.
    ├── tauri.conf.json        Fenster, CSP, Bündel
    ├── capabilities/          Was das Fenster darf
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
    ├── components/            Zeichnen nur
    ├── lib/markdown.ts        Markdown → bereinigtes HTML
    ├── lib/mermaid.ts         Diagramme, nachgeladen
    ├── theme.css              Die EINZIGE Datei mit Farben
    └── app.css                Aufbau und Abstände

scripts/
├── new-project.mjs          Kit zu eigenem Projekt umbenennen
├── package-cli.mjs          dist/teacher-class-cli-<version>-<os>-<arch>.tar.gz
└── tasks.mjs                Was im Justfile keine schlichte Zeile wäre

.claude/agents/              Architektur · Dokumentation · Sicherheit
.github/workflows/           Bau auf drei Systemen, Issues, PR-Durchsicht
```

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
Dafür gibt es die Bau-Workflows unter `.github/workflows/`.

Ergebnisse unter `target/release/bundle/` bzw. `dist/`. Signieren und
Notarisieren: [docs/RELEASE.md](docs/RELEASE.md).

---

## Weiterlesen

| Datei | Inhalt |
|---|---|
| [CLAUDE.md](CLAUDE.md) | Die Regeln. Tauri-2-Fallstricke, festgenagelte Versionen, Schichtregel |
| [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md) | Warum die Schichten so geschnitten sind, wie die IPC-Grenze aussieht |
| [docs/RECIPES.md](docs/RECIPES.md) | npm-Paket einbinden, neuer Befehl, neue Ressource, Plugin, Hintergrundarbeit |
| [docs/DESIGN.md](docs/DESIGN.md) | Aussehen ändern, Farbmarken, Symbole |
| [docs/CLI.md](docs/CLI.md) | Befehle, Rückgabewerte, JSON-Format |
| [docs/AUTOMATION.md](docs/AUTOMATION.md) | Entwicklung über Issues, die drei Prüf-Agents, Einrichtung |
| [docs/RELEASE.md](docs/RELEASE.md) | Bündeln, signieren, notarisieren, veröffentlichen |

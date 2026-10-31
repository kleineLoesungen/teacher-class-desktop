# Ausliefern

## Kurzfassung

```bash
just bundle                   # für diesen Rechner
just bundle-macos-universal   # macOS: Apple Silicon + Intel
just package-cli              # die Kommandozeile als Tarball
```

| System | Ergebnis unter `target/release/bundle/` |
|---|---|
| macOS | `macos/Teacher Class.app`, `dmg/Teacher Class_<version>_<arch>.dmg` |
| Linux | `deb/teacher_class_<version>_<arch>.deb`, `appimage/Teacher Class_<version>_<arch>.AppImage` |
| Windows | `msi/Teacher Class_<version>_x64_en-US.msi`, `nsis/Teacher Class_<version>_x64-setup.exe` |

**Gebaut wird immer auf dem Zielsystem.** Es gibt keine Kreuzübersetzung, die
diesen Namen verdient: WiX (für `.msi`) läuft nur auf Windows, ein `.dmg` nur
auf macOS. Wer alle drei braucht, nimmt die Workflows unter
`.github/workflows/` — siehe [AUTOMATION.md](AUTOMATION.md).

Universal-Bündel landen unter `target/universal-apple-darwin/release/bundle/`.

Die Fassung steht an genau einer Stelle: `version` unter `[workspace.package]`
in der obersten `Cargo.toml`. Tauri liest sie aus `crates/desktop/Cargo.toml`,
das sie erbt. `package.json` hat eine eigene Nummer, die niemand liest.

---

## Über GitHub ausliefern — zwei Wege

```bash
git tag v0.2.0 && git push origin v0.2.0     # am Rechner
```

> **Actions → release → Run workflow**        ← vom Telefon aus

Der zweite Weg nimmt **keine Eingaben**: Der Tag entsteht aus der
Fassungsnummer in `Cargo.toml` und wird vom Workflow selbst angelegt. Die
GitHub-App kann zwar Workflows starten, aber keinen Release anlegen — damit
ist auch der letzte Schritt vom Telefon aus bedienbar.

Beide Wege prüfen dasselbe: Beim Tag-Push muss der Tag zur Fassung in
`Cargo.toml` passen, beim Start von Hand darf es den Tag noch nicht geben.
Ergebnis ist in beiden Fällen ein Release-**Entwurf**. Einzelheiten in
[AUTOMATION.md](AUTOMATION.md).

---

## macOS

### Was im Bündel steckt

```
Teacher Class.app/Contents/
├── Info.plist           aus tauri.conf.json: productName, identifier, Mindestsystem 11.0
├── MacOS/teacher-class-desktop
└── Resources/icon.icns
```

`mainBinaryName` ist ausdrücklich `teacher-class-desktop`. Hieße die Datei wie der
Anzeigename `Teacher Class`, wäre sie auf dem Dateisystem, das Groß- und
Kleinschreibung nicht unterscheidet, dieselbe wie eine daneben gelegte CLI
`teacher-class`.

Die Oberfläche steckt **im Binary** — `ui/dist` wird beim Bau eingebettet.
Ein Bündel ist deshalb nur wenige Megabyte groß; das Webview liefert macOS.

### Die Kommandozeile

Tauri legt keine zweite ausführbare Datei ins Bündel, ohne sie als
„Sidecar" mit Zielplattform im Namen zu verlangen. Die CLI kommt deshalb als
eigener Tarball (`just package-cli`) mit Handbuch und Vervollständigungen.

### `.dmg` auf dem eigenen Rechner

`bundle_dmg.sh` richtet das Fenster des Abbilds über AppleScript im Finder
ein. Hat das Terminal keine Berechtigung, den Finder zu steuern, hängt der
Schritt und bricht ab. Entweder in **Systemeinstellungen → Datenschutz &
Sicherheit → Automation** erlauben, oder ohne Gestaltung bauen:

```bash
CI=true npm run tauri build
```

### Signieren und notarisieren

Ohne Einstellungen signiert Tauri ad hoc. Das genügt auf dem eigenen Rechner.
Sobald das Bündel aber durchs Netz gegangen ist — Release, AirDrop, Anhang —,
hängt eine Quarantäne-Markierung daran, und Gatekeeper verweigert mit
„beschädigt" oder „kann nicht auf Schadsoftware geprüft werden". **Nichts ist
kaputt; es fehlt die Signatur einer Apple-Developer-ID.**

Für den eigenen Rechner gibt es zwei Wege:

1. **Einmal doppelklicken** (wird blockiert), dann
   **Systemeinstellungen → Datenschutz & Sicherheit**, nach unten scrollen,
   **Trotzdem öffnen**. Der sanktionierte Weg.
2. **Markierung entfernen**, wenn sich das Abbild gar nicht erst einhängt:
   ```bash
   xattr -dr com.apple.quarantine ~/Downloads/Teacher Class_<version>_universal.dmg
   ```

Der ältere Rat „Rechtsklick → Öffnen" **funktioniert nicht mehr**; Apple hat
diesen Weg entfernt.

Wer ohnehin den Quelltext hat, umgeht das Ganze: Ein lokal gebautes Bündel
(`just bundle`) trägt keine Quarantäne-Markierung und öffnet sich sofort.

Für die Weitergabe braucht es eine Apple-Developer-ID (kostenpflichtig).
Tauri übernimmt Signieren **und** Notarisieren, wenn diese Umgebungsvariablen
gesetzt sind:

```bash
export APPLE_SIGNING_IDENTITY="Developer ID Application: Dein Name (TEAMID)"
export APPLE_ID="du@example.com"
export APPLE_PASSWORD="app-spezifisches-passwort"
export APPLE_TEAM_ID="TEAMID"
just bundle-macos-universal
```

Das Zertifikat muss dafür im Schlüsselbund liegen.

---

## Linux

### Beim Bauen

```bash
# Debian, Ubuntu
sudo apt install libwebkit2gtk-4.1-dev libxdo-dev libssl-dev \
    libayatana-appindicator3-dev librsvg2-dev build-essential file
```

Fehlen sie, bricht der Bau mit Meldungen über fehlende `.pc`-Dateien ab
(`webkit2gtk-4.1 was not found`), nicht mit einem Rust-Fehler.

**Gebaut wird auf der ältesten Distribution, die unterstützt werden soll.**
Ein Binary, das gegen eine neue glibc gebaut ist, startet auf einer älteren
nicht. Für Ubuntu heißt das: auf 22.04 bauen, nicht auf der neuesten Fassung.

### Auf dem Zielrechner

| Paket | Braucht |
|---|---|
| `.deb` | `libwebkit2gtk-4.1-0` und `libgtk-3-0` — trägt Tauri als Abhängigkeit ein, `apt` zieht sie |
| `.AppImage` | nichts; bringt die Bibliotheken mit (dafür deutlich größer) |

Andere Formate (`.rpm`) in `tauri.conf.json` unter `bundle.targets` ergänzen.

### Wayland, NVIDIA und leere Fenster

WebKitGTK zeichnet auf manchen Kombinationen aus Wayland und
NVIDIA-Treibern ein leeres oder flackerndes Fenster. Abhilfe für Betroffene:

```bash
WEBKIT_DISABLE_DMABUF_RENDERER=1 teacher-class-desktop
```

Wer das dauerhaft braucht, setzt es in `crates/desktop/src/lib.rs` vor dem
Start — nur unter `#[cfg(target_os = "linux")]`, und nur dort.

---

## Windows

### Beim Bauen

Nötig sind die **Visual Studio Build Tools** mit der C++-Arbeitslast (für den
MSVC-Linker, den `rusqlite` mit `bundled` braucht) und **WiX v3** für das
`.msi` — Letzteres holt sich die Tauri-CLI beim ersten Bau selbst.

Meldet `light.exe` einen Fehler, fehlt meist das optionale Windows-Merkmal
**VBSCRIPT**; WiX v3 benutzt es intern.

```powershell
just bundle
```

Erzeugt beides: `.msi` (klassisch, für die Verteilung im Unternehmen) und
`-setup.exe` (NSIS, kleiner und freundlicher für einzelne Benutzer).

### WebView2 — die eine Entscheidung

Anders als macOS und Linux bringt Windows die Engine nicht zwingend mit.
`tauri.conf.json` steht auf `downloadBootstrapper`:

| Modus | Installer größer um | Netz beim Installieren |
|---|---|---|
| `downloadBootstrapper` ← **hier gewählt** | 0 MB | ja, falls WebView2 fehlt |
| `embedBootstrapper` | ~1,8 MB | ja |
| `offlineInstaller` | ~127 MB | nein |

Auf Windows 11 ist WebView2 immer vorhanden, auf gepflegten
Windows-10-Rechnern fast immer. Wer in ein Netz ohne Internet ausliefert,
stellt in `crates/desktop/tauri.conf.json` unter `bundle.windows` auf
`offlineInstaller` um — und bezahlt es mit 127 MB.

**Die Anwendung selbst braucht nie Netz.** Nur der Installer, und auch der nur
dann, wenn WebView2 fehlt.

### Signieren

Unsigniert zeigt Windows beim ersten Start „Der Computer wurde durch Windows
geschützt" (SmartScreen). Weg damit führt über ein Code-Signing-Zertifikat;
Tauri nimmt es über `bundle.windows.certificateThumbprint` entgegen. Ein
EV-Zertifikat baut den SmartScreen-Ruf sofort auf, ein normales erst mit der
Zeit.

### Die Konsole

`crates/desktop/src/main.rs` trägt
`#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]`. Ohne die
Zeile öffnet Windows neben dem Fenster ein Konsolenfenster. Im
Entwicklungsbau bleibt es absichtlich — dort steht das Protokoll aus
`just dev`.

---

## Was die App nicht mitbringt

* **Kein Auto-Update.** `tauri-plugin-updater` braucht einen
  Signaturschlüssel und einen Ort, an dem die Update-Beschreibung liegt. Kommt
  frühestens, wenn Releases regelmäßig erscheinen.
* **Kein Mobil.** Tauri kann iOS und Android; die Symbole dafür löscht
  `just icons` absichtlich.

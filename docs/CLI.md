# Die Kommandozeile

`teacher-class` ist die zweite Schale über derselben Fachlogik wie die Desktop-Anwendung.
Beide arbeiten auf derselben Datenbank.

```bash
# Klassen: Name nur aus Buchstaben und Zahlen, dazu ein vierstelliges Jahr
teacher-class class add 7a 2025
teacher-class class list
teacher-class class edit 8a551a8d --name 7b
teacher-class class rm 8a551a8d --yes          # löscht auch die Schüler

# Schüler: eine Zeile "Nachname, Vorname" je Schüler, alle oder keiner
teacher-class student add 8a551a8d "Muster, Max
Beispiel, Eva"
teacher-class student list 8a551a8d
teacher-class student rm bd10122c --yes

# Prüfungen: Schema (Gruppen mit Punkten) und Bewertungsmatrix (nur „ab“)
teacher-class exam add "Klassenarbeit 1" --class 8a551a8d \
    --group "Aufgabe 1: a=4; b=2,5" --group "Aufgabe 2: a=13,5" \
    --grade 6=0 --grade 3=10 --grade 1=17
teacher-class exam list
teacher-class exam show 3f9c01ab
teacher-class exam export 3f9c01ab --out Klassenarbeit.xlsx
teacher-class exam rm 3f9c01ab --yes
```

## Prüfungen

`exam add` nimmt den Namen und dann:

| Option | Bedeutung |
|---|---|
| `--class KLASSE` | Kennung oder Endstück einer Klasse. Mehrfach möglich, mindestens eine. |
| `--group "Name: Punkt=Max; Punkt=Max"` | Eine Gruppe mit ihren Punkten. Mehrfach möglich. Punkte ganz oder halb, `2,5` und `2.5` gelten gleich. Eine optionale Beschreibung folgt nach `\|`: `a=4\|Rechenweg angeben` (ohne `;` darin). |
| `--grade Note=ab` | Eine Note und der Punktwert, ab dem sie gilt. Mehrfach möglich. Die Noten sind frei benennbar. |
| `--notes` | Zu jedem Prüfkriterium eine Begründung pflegen. Der Excel-Export hat dann eine Bewertungskachel je Schüler statt einer Matrix. |

Das „bis“ jeder Note gibt es nicht als Eingabe; `exam show` zeigt es berechnet
(eine halbe Punktstufe unter dem „ab“ der nächsten Note, bei der höchsten Note
die Gesamtpunktzahl). Die niedrigste Note muss bei 0 beginnen, zwei Noten dürfen
nicht dasselbe „ab“ haben. Verstöße ergeben Rückgabewert 4, ein unlesbares
`--group` oder `--grade` (etwa ohne `:` bzw. `=`) Rückgabewert 2.

```
Klassenarbeit 1  (3f9c01ab)
Klassen: 7a (2025)
Begründungen: nein

Prüfungsschema
  1. Aufgabe 1  (max. 6.5)
    1. a                 4
    2. b                 2.5
  2. Aufgabe 2  (max. 13.5)
    1. a                 13.5
  Gesamt: 20

Bewertungsmatrix
  NOTE             AB     BIS
  6                 0     9.5
  3                10    16.5
  1                17      20
```

`exam score PRÜFUNG SCHÜLER GRUPPE PUNKT [--points P] [--note TEXT]` trägt die
erreichten Punkte und die Begründung eines Schülers zu einem Prüfkriterium ein.
Gruppe und Punkt zählen **ab 1**, wie `exam show` sie nummeriert. Ohne
`--points` werden die Punkte gelöscht, ohne `--note` die Begründung. Eine
Begründung gibt es nur bei Prüfungen mit `--notes`. Ungültige Punkte (nicht
ganz oder halb, über der Höchstpunktzahl), ein unbekanntes Prüfkriterium und
eine Begründung ohne `--notes` ergeben Rückgabewert 4.

`exam results PRÜFUNG --class KLASSE` zeigt je Schüler Gesamtpunkte und Note
(`–`, solange nichts eingetragen ist). Mit `--json` kommen auch die Punkte und
Begründungen je Prüfkriterium.

```
ID        NAME           PUNKTE  NOTE   (max. 20)
7d41e0aa  Beispiel, Eva       0  –
b2c9a113  Muster, Max      17.5  1
```

`exam export --out DATEI` schreibt die Excel-Datei (Blatt „Bewertung“, je Klasse
eins; mit `--notes` je Klasse eine Bewertungskachel pro Schüler) und
überschreibt eine vorhandene. Eingetragene Punkte und Begründungen stehen
darin. Die Datei muss auf `.xlsx` enden, sonst Rückgabewert 4. Bearbeiten einer Prüfung geht in der
Oberfläche; auf der Kommandozeile legt man sie neu an.

## Kennungen

Die Kennung ist eine UUID. In der Bedienung genügt das **Endstück**, so wie
`class list` es anzeigt:

```
ID        KLASSE     SCHÜLER
8a551a8d  7a (2025)  3
bd10122c  8b (2024)  0
```

Warum das Ende und nicht der Anfang: Eine UUIDv7 beginnt mit dem Zeitstempel.
Zwei Datensätze aus derselben Sekunde sähen am Anfang gleich aus.

Ist ein Endstück nicht eindeutig, bricht der Befehl mit Rückgabewert 5 ab und
zeigt die Treffer — nie wird einfach einer davon genommen.

## Rückgabewerte

| Wert | Bedeutung | Beispiel |
|---|---|---|
| 0 | in Ordnung | |
| 1 | allgemeiner Fehler | Platte voll, Datenbank nicht lesbar |
| 2 | falscher Aufruf | `teacher-class class gibtsnicht` — vergibt clap selbst |
| 3 | nicht gefunden | `teacher-class student list ffffffff` |
| 4 | Eingabe ungültig | Klassenname mit Leerzeichen, Schülerzeile ohne Komma, `accent = "blau"` |
| 5 | Kennung nicht eindeutig | `teacher-class student list a` bei mehreren Treffern |

Festgelegt in `crates/cli/src/exit.rs`, geprüft in `crates/cli/tests/cli.rs`.

## `--json`

**Jeder** Befehl kann JSON. Listen sind ein Array, damit `jq '.[]'` ohne Umweg
funktioniert:

```bash
teacher-class class list --json | jq -r '.[] | "\(.id)\t\(.name) (\(.year))"'
```

Zeitstempel stehen in UTC als RFC-3339-Text. Für Menschen rechnet die Ausgabe in
Ortszeit um, für Maschinen nie.

```json
[
  {
    "id": "01a0a3cb-177a-759b-ade1-12c4280a20d3",
    "name": "7a",
    "year": 2025,
    "student_count": 3,
    "created_at": "2026-09-15T06:39:51.162336Z"
  }
]
```

### Fehler als JSON

Gehen auf **stderr**, damit stdout leer bleibt und ein Skript nicht versehentlich
eine Fehlermeldung als Nutzdaten liest:

```json
{
  "error": {
    "code": "validation_failed",
    "fields": [
      { "field": "name", "message": "darf nicht leer sein" }
    ],
    "message": "name: darf nicht leer sein"
  }
}
```

| `error.code` | Rückgabewert | Zusatzfeld |
|---|---|---|
| `validation_failed` | 4 | `fields` |
| `not_found` | 3 | |
| `ambiguous_id` | 5 | `matches` |
| `config_invalid` | 1 | |
| `database_missing` | 1 | Am gewählten Speicherort liegt keine Datenbank, siehe `db` |
| `internal_error` | 1 | |

## In Skripten und Pipelines

```bash
# Schülerliste aus einer Datei (eine Zeile "Nachname, Vorname" je Schüler)
cat schueler.txt | teacher-class student add 8a551a8d -

# Ohne Terminal gibt es keine Rückfrage, sondern einen Fehler.
teacher-class class rm 8a551a8d --yes

# Eigene Verzeichnisse — der saubere Weg für Tests und CI
TEACHER_CLASS_DATA_DIR=/tmp/t/data TEACHER_CLASS_CONFIG_DIR=/tmp/t/config teacher-class class list
```

`class rm` und `student rm` fragen nach, wenn stdin ein Terminal ist. Ist es keins, bricht der
Befehl ab und verweist auf `--yes`. Stillschweigend zu löschen, weil niemand
antworten kann, wäre die falsche Voreinstellung.

## Einstellungen

Dieselbe Datei, die auch die Oberfläche liest und schreibt.

```bash
teacher-class config show
teacher-class config set theme dark          # system | light | dark
teacher-class config set accent "#aa3344"    # #rrggbb
teacher-class config set app_name "Korrekturhilfe"
teacher-class config reset
```

Ein ungültiger Wert wird abgelehnt — der alte Stand bleibt unverändert stehen.

## Speicherort der Datenbank

Ohne Wahl liegt die Datenbank im Datenverzeichnis (siehe unten). Wer sie
woanders haben will — auf einem USB-Stick, in einem synchronisierten Ordner —,
wählt den Ort; er steht dann in der Einstellungsdatei (`db_file`), und die
Oberfläche benutzt ihn ebenso.

```bash
teacher-class db use /media/stick/klassen.sqlite3      # vorhandene Datei nehmen
teacher-class db create /media/stick/klassen.sqlite3   # neue, leere anlegen
teacher-class db reset                                 # zurück zum Standardort
```

`db create` überschreibt nie eine vorhandene Datei (Rückgabewert 4), `db use`
verlangt eine vorhandene (ebenso 4). `config set db_file` gibt es nicht; `config
reset` lässt den Ort stehen.

Liegt am gewählten Ort keine Datenbank mehr (Stick nicht eingesteckt), legt
kein Befehl still eine neue an: Sie enden mit `database_missing` und
Rückgabewert 1. `db use`, `db create` und `db reset` laufen trotzdem — sie sind
der Ausweg. Am Standardort wird die Datenbank beim ersten Start angelegt.

## Wo liegt was

```bash
teacher-class paths
teacher-class paths --json | jq -r .db_file    # mit gewähltem Ort, falls es einen gibt
```

| | macOS | Linux | Windows |
|---|---|---|---|
| Daten | `~/Library/Application Support/teacher-class` | `~/.local/share/teacher-class` | `%APPDATA%\teacher-class\data` |
| Konfiguration | `~/Library/Application Support/teacher-class` | `~/.config/teacher-class` | `%APPDATA%\teacher-class\config` |

Überschreiben mit `--data-dir` / `--config-dir` oder den Umgebungsvariablen
`TEACHER_CLASS_DATA_DIR` / `TEACHER_CLASS_CONFIG_DIR`.

## Protokoll

Geht auf stderr, nie auf stdout.

```bash
teacher-class -v class list          # info
teacher-class -vv class list         # debug
TEACHER_CLASS_LOG=teacher-class=trace teacher-class class list
```

## Vervollständigung und Handbuch

```bash
teacher-class completions zsh  > ~/.zfunc/_teacher-class
teacher-class completions bash > /etc/bash_completion.d/teacher-class
teacher-class man --out ~/.local/share/man/man1
```

Beides erzeugt die Anwendung aus ihrer eigenen Befehlsstruktur — es kann also
nicht veralten. Der Tarball aus `just package-cli` enthält beides fertig.

Diese beiden Befehle brauchen **keine** Datenbank; sie laufen auch dort, wo das
Datenverzeichnis nicht beschreibbar ist.

# Die Kommandozeile

`starter` ist die zweite Schale über derselben Fachlogik wie die Desktop-Anwendung.
Beide arbeiten auf derselben Datenbank.

```bash
starter note add "Einkauf" --body "Milch, Brot"
starter note list
starter note show 8a551a8d
starter note edit 8a551a8d --title "Großeinkauf"
starter note rm 8a551a8d --yes
```

## Kennungen

Die Kennung ist eine UUID. In der Bedienung genügt das **Endstück**, so wie
`note list` es anzeigt:

```
ID        TITEL    GEÄNDERT
8a551a8d  Zebra    15.09.2026 08:40
bd10122c  Bauplan  15.09.2026 08:40
```

Warum das Ende und nicht der Anfang: Eine UUIDv7 beginnt mit dem Zeitstempel.
Zwei Notizen aus derselben Sekunde sähen am Anfang gleich aus.

Ist ein Endstück nicht eindeutig, bricht der Befehl mit Rückgabewert 5 ab und
zeigt die Treffer — nie wird einfach einer davon genommen.

## Rückgabewerte

| Wert | Bedeutung | Beispiel |
|---|---|---|
| 0 | in Ordnung | |
| 1 | allgemeiner Fehler | Platte voll, Datenbank nicht lesbar |
| 2 | falscher Aufruf | `starter note gibtsnicht` — vergibt clap selbst |
| 3 | nicht gefunden | `starter note show ffffffff` |
| 4 | Eingabe ungültig | leerer Titel, `accent = "blau"` |
| 5 | Kennung nicht eindeutig | `starter note show a` bei mehreren Treffern |

Festgelegt in `crates/cli/src/exit.rs`, geprüft in `crates/cli/tests/cli.rs`.

## `--json`

**Jeder** Befehl kann JSON. Listen sind ein Array, damit `jq '.[]'` ohne Umweg
funktioniert:

```bash
starter note list --json | jq -r '.[] | "\(.id)\t\(.title)"'
```

Zeitstempel stehen in UTC als RFC-3339-Text. Für Menschen rechnet die Ausgabe in
Ortszeit um, für Maschinen nie.

```json
[
  {
    "id": "01a0a3cb-177a-759b-ade1-12c4280a20d3",
    "title": "Bauplan",
    "body": "Regal",
    "created_at": "2026-09-15T06:39:51.162336Z",
    "updated_at": "2026-09-15T06:39:51.162336Z"
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
      { "field": "title", "message": "darf nicht leer sein" }
    ],
    "message": "title: darf nicht leer sein"
  }
}
```

| `error.code` | Rückgabewert | Zusatzfeld |
|---|---|---|
| `validation_failed` | 4 | `fields` |
| `not_found` | 3 | |
| `ambiguous_id` | 5 | `matches` |
| `config_invalid` | 1 | |
| `internal_error` | 1 | |

## In Skripten und Pipelines

```bash
# Text aus einer Datei
cat notiz.md | starter note add "Aus der Datei" --body -

# Ohne Terminal gibt es keine Rückfrage, sondern einen Fehler.
starter note rm 8a551a8d --yes

# Eigene Verzeichnisse — der saubere Weg für Tests und CI
STARTER_DATA_DIR=/tmp/t/data STARTER_CONFIG_DIR=/tmp/t/config starter note list
```

`note rm` fragt nach, wenn stdin ein Terminal ist. Ist es keins, bricht der
Befehl ab und verweist auf `--yes`. Stillschweigend zu löschen, weil niemand
antworten kann, wäre die falsche Voreinstellung.

## Einstellungen

Dieselbe Datei, die auch die Oberfläche liest und schreibt.

```bash
starter config show
starter config set theme dark          # system | light | dark
starter config set accent "#aa3344"    # #rrggbb
starter config set app_name "Notizbuch"
starter config reset
```

Ein ungültiger Wert wird abgelehnt — der alte Stand bleibt unverändert stehen.

## Wo liegt was

```bash
starter paths
starter paths --json | jq -r .db_file
```

| | macOS | Linux | Windows |
|---|---|---|---|
| Daten | `~/Library/Application Support/starter` | `~/.local/share/starter` | `%APPDATA%\starter\data` |
| Konfiguration | `~/Library/Application Support/starter` | `~/.config/starter` | `%APPDATA%\starter\config` |

Überschreiben mit `--data-dir` / `--config-dir` oder den Umgebungsvariablen
`STARTER_DATA_DIR` / `STARTER_CONFIG_DIR`.

## Protokoll

Geht auf stderr, nie auf stdout.

```bash
starter -v note list          # info
starter -vv note list         # debug
STARTER_LOG=starter=trace starter note list
```

## Vervollständigung und Handbuch

```bash
starter completions zsh  > ~/.zfunc/_starter
starter completions bash > /etc/bash_completion.d/starter
starter man --out ~/.local/share/man/man1
```

Beides erzeugt die Anwendung aus ihrer eigenen Befehlsstruktur — es kann also
nicht veralten. Der Tarball aus `just package-cli` enthält beides fertig.

Diese beiden Befehle brauchen **keine** Datenbank; sie laufen auch dort, wo das
Datenverzeichnis nicht beschreibbar ist.

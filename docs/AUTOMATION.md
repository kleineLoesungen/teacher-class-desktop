# Entwicklung über Issues

Der Gedanke: **Die Arbeit beginnt in einem Issue und endet in einem Merge —
alles davon vom Telefon aus bedienbar.** Kein Rechner nötig, bis man einen
will.

```
 Telefon                     GitHub Actions                      Telefon
 ───────                     ──────────────                      ───────
 Issue schreiben
 „@claude mach X"   ──────▶  claude-issue.yml
                             Claude liest CLAUDE.md,
                             ändert Code, lässt
                             `just check` laufen,
                             öffnet einen PR        ──────▶  PR kommt an
                                                             ▼
                                                      „Approve workflows
                                                        to run" tippen
                                                             │
                             check.yml      ◀────────────────┤
                             (macOS · Linux · Windows)       │
                             durchsicht.yml ◀────────────────┘
                             (3 Prüf-Agents)
                                     │
                                     ▼
                             3 Kommentare + grüne Haken  ──▶  lesen, mergen
```

---

## Einrichtung

Ohne diesen Schritt tut `@claude` nichts — die Workflows überspringen sich
dann stillschweigend und hinterlassen einen Hinweis im Lauf statt eines
roten Hakens.

**Das Kit selbst hat bewusst keinen Token hinterlegt.** Ein Secret gehört in
ein Projekt, nicht in eine Vorlage. Die Schritte unten macht man einmal je
abgeleitetem Projekt.

### 1. Token erzeugen

An einem Rechner mit der Claude-CLI, einmal überhaupt:

```bash
claude setup-token
```

Der Token hängt an **deinem Abo** (Pro, Max, Team oder Enterprise), nicht am
Repository — **derselbe Token passt in alle deine Projekte**. Kopiere ihn
beim Erzeugen gleich weg; ob er später noch einmal abrufbar ist, verlass dich
nicht darauf.

### 2. Als Secret hinterlegen

*Settings → Secrets and variables → Actions → New repository secret*

| | |
|---|---|
| Name | `CLAUDE_CODE_OAUTH_TOKEN` |
| Wert | die Ausgabe von `claude setup-token` |

Das geht auch im Browser auf dem Telefon — den Token aus dem Passwortspeicher
einfügen.

### 3. Probe

Ein Issue anlegen: *„@claude trage in die README eine Zeile ein, dass dieses
Projekt aus dem Kit stammt."* Innerhalb einer Minute sollte ein Lauf unter
*Actions* erscheinen.

---

## Wer Claude auslösen darf

**Nur der Repo-Inhaber, und nur mit `@claude` im Text.** Beides muss
zutreffen — das ist kein Versehen, sondern die Absicht:

| Schloss | Wo |
|---|---|
| Kommentar kommt vom Repo-Inhaber | die `if`-Bedingung in `claude-issue.yml` |
| Text enthält `@claude` | Auslösewort der Action |
| Konto hat Schreibrecht | Voreinstellung von `claude-code-action` |

Das Repository ist öffentlich: **Jeder kann Issues schreiben und
kommentieren.** Ohne die erste Bedingung könnte ein Fremder mit einem
`@claude`-Kommentar einen selbst geschriebenen Auftrag in deiner CI
ausführen lassen — auf Kosten deines Abo-Kontingents. `claude-code-action`
kennt dafür den Schalter `allowed_non_write_users`; dieses Kit setzt ihn
**nicht**, und die Doku der Action nennt ihn ausdrücklich ein erhebliches
Risiko.

Fremde Kommentare werden stillschweigend übergangen. Es gibt keine Antwort,
keinen fehlgeschlagenen Lauf, keine Benachrichtigung.

---

## Der eine Fingertipp

Einen Pull Request, den ein Workflow mit dem `GITHUB_TOKEN` öffnet, stellt
GitHub bewusst auf **„approval-required"**. Im PR erscheint ein Banner:

> **Approve workflows to run**

Erst danach laufen `check` und `durchsicht`. Das ist kein Fehler, den man
umgehen sollte, sondern die Stelle, an der ein Mensch zustimmt, dass Prüfung
und Bau auf von der KI geschriebenem Code loslaufen. Ein Tipp im Browser oder
in der GitHub-App.

*Wer das nicht will,* trägt in `claude-issue.yml` ein Personal Access Token
als `github_token` ein. Dann entfällt die Rückfrage — und dafür läuft alles
unbeaufsichtigt, das Token läuft irgendwann ab, und seine Reichweite ist
größer als die des `GITHUB_TOKEN`. Dieses Kit hat sich bewusst anders
entschieden.

---

## Die drei Prüf-Agents

Auf **jedem** PR (außer Entwürfen und PRs aus Forks) laufen drei Durchsichten
nebeneinander. Ihre Prüflisten stehen als lesbare Dateien im Repo:

| Agent | Datei | Sieht auf |
|---|---|---|
| 🏛 Architektur | `.claude/agents/architektur.md` | Schichtregel, IPC-Grenze, Tauri-Fallstricke, unnötige Komplexität |
| 📖 Dokumentation | `.claude/agents/dokumentation.md` | Ist die Doku nach der Änderung noch wahr? |
| 🔒 Sicherheit | `.claude/agents/sicherheit.md` | CSP, Capabilities, Bereinigung von Benutzertext, SQL, Geheimnisse |

Jeder schreibt **genau einen** Kommentar mit einem von drei Urteilen:
**in Ordnung** · **Anmerkungen** · **Einspruch**.

Drei Dinge, die bewusst so sind:

* **Alle drei laufen auf Sonnet.** Was sich mechanisch prüfen lässt, prüft
  schon `just check` — `layering.rs` findet `invoke(` in einer Komponente,
  `ipc_contract.rs` die fehlende Befehlszeile. Den Agents bleibt das Urteil
  gegen eine benannte Prüfliste, und das kann Sonnet. Das Modell steht als
  eine Zeile in jeder Agent-Datei.
* **Sie dürfen nichts ändern.** `--allowedTools` lässt Lesen und einen
  PR-Kommentar zu, sonst nichts. Eine Durchsicht, die selbst committet, ist
  keine Durchsicht.
* **„in Ordnung" ist eine erwünschte Antwort.** In jeder Agent-Datei steht,
  was sie *nicht* melden soll. Ein Agent, der immer etwas findet, wird nach
  drei PRs nicht mehr gelesen.

Die Urteile sind **Rat, kein Riegel**. Blockieren tut nur `check` — also das,
was auch lokal grün sein muss.

---

## Die Workflows im Überblick

| Datei | Wann | Was |
|---|---|---|
| `check.yml` | Push nach `main`, jeder PR | `just fmt-check lint test` auf macOS, Linux und Windows; `just deny` einmal auf Linux |
| `durchsicht.yml` *(`claude-review.yml`)* | jeder PR | die drei Agents |
| `claude-issue.yml` | Issue und Issue-Kommentar vom Inhaber mit `@claude` | Umsetzung, PR |
| `release.yml` | Tag `v*` | Bündel für alle drei Systeme als Release-**Entwurf** |

`check.yml` ist das Sicherheitsnetz für Windows und Linux: Entwickelt wird auf
macOS, und ein Unterschied, der nur dort nicht auffällt, fiele sonst niemandem
auf.

---

## Was es verbraucht

* **Actions-Minuten:** in einem öffentlichen Repository kostenlos, auch für
  macOS- und Windows-Runner. In einem **privaten** zählt macOS zehnfach und
  Windows doppelt — dort lohnt es, die Matrix in `check.yml` auf Linux zu
  kürzen und die anderen beiden erst beim Tag zu bauen.
* **Abo-Kontingent:** Ein Issue-Lauf ist eine Claude-Sitzung, jeder PR
  zusätzlich drei kurze. Wer das zu viel findet, nimmt in
  `claude-review.yml` Agents aus der `matrix` heraus.

---

## Wenn es nicht läuft

| Zu sehen | Grund |
|---|---|
| Gar kein Lauf nach `@claude` | Kommentar nicht vom Repo-Inhaber, oder `@claude` fehlt |
| Gar kein Lauf, obwohl du der Inhaber bist | Liegt das Repository unter einer **Organisation**? Dann ist `github.repository_owner` deren Name und nie dein Benutzername. In `claude-issue.yml` den eigenen Namen eintragen — der Kommentar dort sagt, wo |
| Lauf grün, aber Claude hat nichts getan | Kein Token hinterlegt. Im Lauf steht ein Hinweis „übersprungen" |
| Lauf scheitert an der Anmeldung | Token abgelaufen oder falsch eingefügt — neu mit `claude setup-token` |
| PR da, aber keine Haken und keine Kommentare | Der Freigabe-Tipp fehlt noch (siehe oben) |
| `durchsicht` übersprungen | PR ist ein Entwurf oder kommt aus einem Fork |
| `check` rot nur auf Windows | Genau der Fall, für den die Matrix da ist. `crates/core/tests/layering.rs` nennt die üblichen Ursachen |

---

## In einem Projekt aus dem Template

`node scripts/new-project.mjs` nimmt `.github/` und `.claude/` mit — das neue
Projekt hat alles sofort.

**Was es nicht mitnimmt, ist der Token.** Der hängt am Abo einer Person und
hat in einer Vorlage nichts zu suchen. Jedes Projekt braucht seinen eigenen
Eintrag `CLAUDE_CODE_OAUTH_TOKEN` — Schritt 2 oben, mit demselben Token.

Bis dahin laufen die beiden Claude-Workflows ins Leere, aber sauber: Sie
überspringen sich und schreiben einen Hinweis in den Lauf. Keine roten Haken
an einem frischen Projekt.

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

**Zwei Dinge werden gebraucht, nicht eines:** die Claude-GitHub-App auf dem
Repository **und** den Token als Secret. Fehlt der Token, überspringen sich
die Workflows stillschweigend. Fehlt die App, laufen sie an und scheitern
mit `Claude Code is not installed on this repository` — die Inhaber-Sperre
und das Auslösewort greifen also schon, nur der Token-Tausch nicht.

**Das Kit selbst hat bewusst keines von beidem.** Ein Secret gehört in ein
Projekt, nicht in eine Vorlage. Die Schritte unten macht man einmal je
abgeleitetem Projekt.

### 1. Die Claude-GitHub-App installieren

<https://github.com/apps/claude> → **Install** → dein Konto → **Only select
repositories** → das Projekt auswählen.

Du musst Verwalter des Repositorys sein. Die App liefert die Berechtigung,
mit der die Action ihren OIDC-Token gegen einen Zugriffstoken tauscht; ohne
sie nützt auch ein gültiges Abo-Token nichts.

*(Wer die offizielle App nicht will, kann eine eigene GitHub-App anlegen und
sie über den Eingabewert `github_token` einhängen — mehr Arbeit, dafür
niemand Drittes mit Zugriff.)*

### 2. Token erzeugen

An einem Rechner mit der Claude-CLI, einmal überhaupt:

```bash
claude setup-token
```

Der Token hängt an **deinem Abo** (Pro, Max, Team oder Enterprise), nicht am
Repository — **derselbe Token passt in alle deine Projekte**. Kopiere ihn
beim Erzeugen gleich weg; ob er später noch einmal abrufbar ist, verlass dich
nicht darauf.

### 3. Als Secret hinterlegen

*Settings → Secrets and variables → Actions → New repository secret*

| | |
|---|---|
| Name | `CLAUDE_CODE_OAUTH_TOKEN` |
| Wert | die Ausgabe von `claude setup-token` |

Das geht auch im Browser auf dem Telefon — den Token aus dem Passwortspeicher
einfügen.

### 4. Probe

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
| `token-pruefen.yml` | von Hand | Sagt, ob der Token im Secret trägt |
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
| `Claude Code is not installed on this repository` | Die GitHub-App fehlt — Schritt 1. Der Token ist in Ordnung |
| `Claude execution failed: result is_error:true`, Abbruch nach ~2 s, `total_cost_usd: 0` | Der Token wird abgelehnt. 401 oder 429 — siehe unten |
| PR da, aber keine Haken und keine Kommentare | Der Freigabe-Tipp fehlt noch (siehe oben) |
| `durchsicht` übersprungen | PR ist ein Entwurf oder kommt aus einem Fork |
| `check` rot nur auf Windows | Genau der Fall, für den die Matrix da ist. `crates/core/tests/layering.rs` nennt die üblichen Ursachen |

---

### Wenn der Token abgelehnt wird

Sieht so aus:

```json
{"type":"result","subtype":"success","is_error":true,
 "duration_ms":1996,"num_turns":1,"total_cost_usd":0,"modelUsage":{}}
```

Kosten null und leeres `modelUsage` heißen: **kein einziger Modellaufruf kam
zustande.** Die Action verschluckt den eigentlichen Grund — bekannte offene
Ausgabe [claude-code-action#1614](https://github.com/anthropics/claude-code-action/issues/1614).
Dahinter stecken zwei verschiedene Fälle, die **gegensätzliche** Maßnahmen
verlangen:

| | |
|---|---|
| **401** | Der Token ist ungültig, abgelaufen oder beim Einfügen beschädigt → neu erzeugen |
| **429** | Das Abo-Kontingent ist erschöpft → ein neuer Token ändert **nichts**, nur Warten oder ein API-Schlüssel hilft |

**Häufigster Fall ist ein Leerzeichen.** Beim Kopieren rutscht leicht eines
mit, und im Secret-Feld sieht man es nicht. Die Workflows prüfen das
inzwischen selbst und brechen mit Klartext ab, statt dich die zwei Sekunden
deuten zu lassen.

#### Wenn der Token in Ordnung ist und es trotzdem scheitert

Dann braucht es die verschwiegene Fehlermeldung. Variable
**`CLAUDE_DEBUG` = `true`** setzen (*Settings → Secrets and variables →
Actions → **Variables***), den Lauf wiederholen, Meldung lesen,
**Variable wieder auf `false`**.

In einem öffentlichen Repository ist die volle Ausgabe danach für jeden
lesbar — auch Dateiinhalte, die Claude liest. Deshalb anlassbezogen
einschalten, nie dauerhaft.

#### Den Token im Secret prüfen lassen

Statt zu raten: **Actions → „token prüfen" → Run workflow.**

Der Lauf nimmt den Token aus dem Secret — also genau den, den auch die
Action benutzt — und sagt in Klartext, was der Dienst dazu meint:

| Ausgabe | Bedeutung |
|---|---|
| *Kein Token* | Das Secret fehlt. Schritt 3 |
| *Token beschädigt* | Leerzeichen oder Zeilenumbruch. Neu einfügen |
| *401 — Token abgelehnt* | Ungültig oder abgelaufen. `claude setup-token`, Secret ersetzen |
| *429 — Kontingent erschöpft* | Ein **neuer** Token ändert nichts |
| *Token in Ordnung* | Scheitert Claude trotzdem, liegt es nicht am Token |

Der Lauf hat zwei Aufgaben, und ihr Zusammenspiel ist die eigentliche
Auskunft:

| `token` | `action` | Bedeutung |
|---|---|---|
| grün | grün | Einrichtung in Ordnung. Ein Fehler liegt am Auftrag, nicht am Aufbau |
| rot | — | Das Secret selbst. Meldung oben lesen |
| grün | rot | Der Token ist gut, erreicht aber die Claude-CLI im Runner nicht (`Not logged in · Please run /login`). Das liegt **nicht** an diesem Projekt — siehe [claude-code-action#676](https://github.com/anthropics/claude-code-action/issues/676) |

Die Aufgabe `action` ruft dieselbe Action wie `claude-issue.yml`, aber ohne
alles Drumherum: keine Werkzeugkette, kein Cache, kein Job-`env`. Läuft sie
und der große Workflow nicht, liegt es an einem der weggelassenen Schritte —
und man weiß, wo zu suchen ist.

Der Token wird dabei nie ausgegeben, nur seine Länge und die Antwort des
Dienstes. Den Lauf kann nur der Repo-Inhaber starten.

Wer das verwechselt, erzeugt im zweiten Fall endlos neue Token, die alle
dasselbe tun. Deshalb erst fragen, dann handeln — **am eigenen Rechner**, der
Token gehört nirgendwo anders hin.

Zuerst prüfen, dass die Variable überhaupt gefüllt ist. Ein leeres
`Bearer ` ergibt nämlich ebenfalls 401, und man misst den eigenen
Tippfehler statt des Fehlers:

```bash
export CLAUDE_CODE_OAUTH_TOKEN='…'
echo ${#CLAUDE_CODE_OAUTH_TOKEN}      # muss eine dreistellige Zahl sein, nicht 0
```

Dann erst:

```bash
curl -s -D- -o /dev/null https://api.anthropic.com/v1/messages \
  -H "authorization: Bearer $CLAUDE_CODE_OAUTH_TOKEN" \
  -H "anthropic-version: 2023-06-01" \
  -H "anthropic-beta: oauth-2025-04-20" \
  -H "content-type: application/json" \
  -d '{"model":"claude-sonnet-5","max_tokens":16,
       "system":"You are Claude Code, Anthropic'"'"'s official CLI for Claude.",
       "messages":[{"role":"user","content":"say OK"}]}'
```

Die `system`-Zeile ist nicht schmückend: Ohne sie weist der Dienst
Abo-Token ab, und man misst den falschen Fehler.

* **`HTTP/2 429`** mit Kopfzeilen `anthropic-ratelimit-unified-…` → Kontingent.
* **`HTTP/2 401`** ohne solche Kopfzeilen → echte Ablehnung, neuen Token holen.
* **`HTTP/2 200`** → der Token ist in Ordnung, der Fehler liegt woanders.

**Wenn es das Kontingent war**, ist die Frage nicht technisch, sondern eine
Entscheidung: warten, oder die Workflows auf einen API-Schlüssel umstellen
(`anthropic_api_key` statt `claude_code_oauth_token`). Der wird getrennt
abgerechnet und hängt nicht am Abo — in der verlinkten Ausgabe ist das der
Weg, auf dem die meisten gelandet sind.

---

## In einem Projekt aus dem Template

`node scripts/new-project.mjs` nimmt `.github/` und `.claude/` mit — das neue
Projekt hat alles sofort.

**Was es nicht mitnimmt, sind App und Token.** Beide hängen an einem Konto
und haben in einer Vorlage nichts zu suchen. Jedes Projekt braucht die
GitHub-App (Schritt 1) und seinen eigenen Eintrag `CLAUDE_CODE_OAUTH_TOKEN`
(Schritt 3, mit demselben Token).

Bis dahin laufen die beiden Claude-Workflows ins Leere, aber sauber: Sie
überspringen sich und schreiben einen Hinweis in den Lauf. Keine roten Haken
an einem frischen Projekt.

// Die einzige Komponente, die api.ts benutzt.
//
// Hier liegt der Zustand der Oberfläche, und hier wird aus einem Wunsch einer
// Komponente ein Befehl an Rust. Das Gegenstück zu app.rs im egui-Kit:
//
//   Komponente  ──onSave()──▶  App.tsx  ──api.notes.update()──▶  Rust
//
// Fachliche Regeln stehen hier nicht. Ob ein Titel leer sein darf, entscheidet
// crates/core — hier wird nur angezeigt, was von dort zurückkommt.

import { useCallback, useEffect, useState } from "react";
import { ApiError, api } from "./api";
import type { AppInfo, Config, Note, NoteInput, Order } from "./bindings";
import { ConfirmDialog } from "./components/ConfirmDialog";
import { EmptyState } from "./components/EmptyState";
import { type EditorMode, NoteEditor } from "./components/NoteEditor";
import { NoteList } from "./components/NoteList";
import { SettingsDialog } from "./components/SettingsDialog";
import { applyTheme, useIsDark } from "./lib/theme";

type Selection = { kind: "none" } | { kind: "new" } | { kind: "note"; id: string };

const EMPTY: NoteInput = { title: "", body: "" };

export const EXAMPLE_NOTE: NoteInput = {
  title: "Beispiel: Ablauf",
  body: `# Wie eine Änderung gespeichert wird

Jeder Knopfdruck geht **denselben Weg** — ob aus der Oberfläche oder von der Kommandozeile.

\`\`\`mermaid
flowchart TD
    UI[Komponente] -->|Rückruf| App[App.tsx]
    App -->|api.ts| IPC{{Tauri IPC}}
    IPC --> Cmd[commands.rs]
    CLI[starter CLI] --> Core
    Cmd --> Core[(starter-core)]
\`\`\`

\`\`\`mermaid
sequenceDiagram
    participant W as Webview
    participant R as Rust
    W->>R: note_update { id, input }
    R-->>W: Note oder ApiError
\`\`\`

Mehr über Mermaid: [mermaid.js.org](https://mermaid.js.org)
`,
};

type Props = {
  /** Wartezeit der Vorschau. Tests setzen 0. */
  previewDelayMs?: number;
};

export function App({ previewDelayMs }: Props) {
  const [notes, setNotes] = useState<Note[]>([]);
  const [order, setOrder] = useState<Order>("recent");
  const [selection, setSelection] = useState<Selection>({ kind: "none" });
  const [draft, setDraft] = useState<NoteInput>(EMPTY);
  const [saved, setSaved] = useState<NoteInput>(EMPTY);
  const [mode, setMode] = useState<EditorMode>("split");
  const [saving, setSaving] = useState(false);

  const [config, setConfig] = useState<Config | null>(null);
  const [info, setInfo] = useState<AppInfo | null>(null);
  const [dialog, setDialog] = useState<"settings" | "delete" | null>(null);

  // Zwei Sorten Fehler, wie überall im Kit: mit Feldern an die Felder,
  // ohne Felder als Meldung oben.
  const [fieldError, setFieldError] = useState<ApiError | null>(null);
  const [banner, setBanner] = useState<string | null>(null);

  const dark = useIsDark(config?.theme ?? "system");
  const dirty = draft.title !== saved.title || draft.body !== saved.body;

  /** Ein fehlgeschlagener Befehl: ans Feld oder nach oben. */
  const fail = useCallback((raw: unknown) => {
    const err = ApiError.from(raw);
    if (err.isValidation) {
      setFieldError(err);
    } else {
      setBanner(err.message);
    }
  }, []);

  const reload = useCallback(
    async (nextOrder: Order = order) => {
      try {
        setNotes(await api.notes.list(nextOrder));
      } catch (raw) {
        fail(raw);
      }
    },
    [order, fail],
  );

  // --- Start ---------------------------------------------------------------
  // biome-ignore lint/correctness/useExhaustiveDependencies: nur einmal beim Start
  useEffect(() => {
    void (async () => {
      try {
        const [loaded, appInfo] = await Promise.all([api.config.get(), api.appInfo()]);
        setConfig(loaded);
        setInfo(appInfo);
      } catch (raw) {
        fail(raw);
      }
      await reload();
    })();
  }, []);

  useEffect(() => {
    if (!config) return;
    applyTheme(config);
    document.title = config.app_name;
    api.setWindowTitle(config.app_name).catch(() => {
      // Ohne Fenster (Tests, Browser) gibt es keinen Titel zu setzen.
    });
  }, [config]);

  // --- Notizen -------------------------------------------------------------

  function open(next: Selection, input: NoteInput) {
    setSelection(next);
    setDraft(input);
    setSaved(input);
    setFieldError(null);
  }

  async function save(): Promise<boolean> {
    setSaving(true);
    try {
      const note =
        selection.kind === "note"
          ? await api.notes.update(selection.id, draft)
          : await api.notes.create(draft);
      // Rust hat aufgeräumt (Leerzeichen am Rand) — das Ergebnis übernehmen.
      open({ kind: "note", id: note.id }, { title: note.title, body: note.body });
      await reload();
      return true;
    } catch (raw) {
      fail(raw);
      return false;
    } finally {
      setSaving(false);
    }
  }

  /** Wechseln speichert. Scheitert das Speichern, bleibt die Notiz offen. */
  async function leaveCurrent(): Promise<boolean> {
    if (selection.kind === "none" || !dirty) return true;
    return save();
  }

  async function select(id: string) {
    if (selection.kind === "note" && selection.id === id) return;
    if (!(await leaveCurrent())) return;
    try {
      const note = await api.notes.get(id);
      open({ kind: "note", id }, { title: note.title, body: note.body });
    } catch (raw) {
      fail(raw);
    }
  }

  async function createNew() {
    if (!(await leaveCurrent())) return;
    open({ kind: "new" }, EMPTY);
    setMode("split");
  }

  async function createExample() {
    try {
      const note = await api.notes.create(EXAMPLE_NOTE);
      open({ kind: "note", id: note.id }, { title: note.title, body: note.body });
      setMode("split");
      await reload();
    } catch (raw) {
      fail(raw);
    }
  }

  async function remove() {
    setDialog(null);
    if (selection.kind !== "note") return;
    try {
      await api.notes.remove(selection.id);
      open({ kind: "none" }, EMPTY);
      await reload();
    } catch (raw) {
      fail(raw);
    }
  }

  async function changeOrder(next: Order) {
    setOrder(next);
    await reload(next);
  }

  async function saveConfig(next: Config) {
    try {
      setConfig(await api.config.set(next));
      setFieldError(null);
      setDialog(null);
    } catch (raw) {
      fail(raw);
    }
  }

  function openLink(url: string) {
    api.openExternal(url).catch(fail);
  }

  // --- Tastatur ------------------------------------------------------------
  // Cmd auf macOS, Strg auf Linux.
  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      if (!(e.metaKey || e.ctrlKey) || dialog) return;
      if (e.key === "s" && selection.kind !== "none") {
        e.preventDefault();
        void save();
      } else if (e.key === "n") {
        e.preventDefault();
        void createNew();
      }
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  });

  // --- Zeichnen ------------------------------------------------------------

  const fieldMessage = (field: string) => fieldError?.fieldMessage(field);

  return (
    <div className="app">
      <aside className="sidebar">
        <div className="brand">{config?.app_name ?? " "}</div>
        <NoteList
          notes={notes}
          selectedId={selection.kind === "note" ? selection.id : null}
          order={order}
          onSelect={(id) => void select(id)}
          onCreate={() => void createNew()}
          onOrderChange={(next) => void changeOrder(next)}
        />
        <button
          type="button"
          className="ghost settings-button"
          disabled={!config}
          onClick={() => {
            setFieldError(null);
            setDialog("settings");
          }}
        >
          Einstellungen
        </button>
      </aside>

      <main className="main">
        {banner && (
          <div className="banner" role="alert">
            <span>{banner}</span>
            <button type="button" className="ghost" onClick={() => setBanner(null)}>
              Schließen
            </button>
          </div>
        )}

        {selection.kind === "none" ? (
          <EmptyState
            hasNotes={notes.length > 0}
            onCreate={() => void createNew()}
            onCreateExample={() => void createExample()}
          />
        ) : (
          <NoteEditor
            draft={draft}
            isNew={selection.kind === "new"}
            dirty={dirty}
            saving={saving}
            mode={mode}
            dark={dark}
            fieldMessage={dialog ? () => undefined : fieldMessage}
            onChange={setDraft}
            onModeChange={setMode}
            onSave={() => void save()}
            onDelete={() => setDialog("delete")}
            onOpenLink={openLink}
            {...(previewDelayMs !== undefined && { previewDelayMs })}
          />
        )}
      </main>

      {dialog === "settings" && config && (
        <SettingsDialog
          config={config}
          info={info}
          fieldMessage={fieldMessage}
          onSave={(next) => void saveConfig(next)}
          onClose={() => {
            setFieldError(null);
            setDialog(null);
          }}
        />
      )}

      {dialog === "delete" && (
        <ConfirmDialog
          title="Notiz löschen"
          message={`„${saved.title}" wird endgültig gelöscht.`}
          confirmLabel="Endgültig löschen"
          onConfirm={() => void remove()}
          onCancel={() => setDialog(null)}
        />
      )}
    </div>
  );
}

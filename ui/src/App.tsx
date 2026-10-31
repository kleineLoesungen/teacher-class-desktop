// Die einzige Komponente, die api.ts benutzt.
//
// Hier liegt der Zustand der Oberfläche, und hier wird aus einem Wunsch einer
// Komponente ein Befehl an Rust:
//
//   Komponente  ──onSave()──▶  App.tsx  ──api.classes.update()──▶  Rust
//
// Fachliche Regeln stehen hier nicht. Ob ein Klassenname gültig ist oder eine
// Schülerzeile das Format `Nachname, Vorname` hat, entscheidet crates/core —
// hier wird nur angezeigt, was von dort zurückkommt.

import { useCallback, useEffect, useState } from "react";
import { ApiError, api } from "./api";
import type { AppInfo, Class, ClassInput, Config, Student } from "./bindings";
import { ClassEditor } from "./components/ClassEditor";
import { ClassList } from "./components/ClassList";
import { ConfirmDialog } from "./components/ConfirmDialog";
import { EmptyState } from "./components/EmptyState";
import { SettingsDialog } from "./components/SettingsDialog";
import { classLabel } from "./lib/format";
import { applyTheme } from "./lib/theme";

type Selection = { kind: "none" } | { kind: "new" } | { kind: "class"; id: string };

type Dialog = "settings" | "delete-class" | { student: Student } | null;

/** Eine neue Klasse beginnt im laufenden Jahr; das Jahr ist nur ein Vorschlag. */
function emptyDraft(): ClassInput {
  return { name: "", year: new Date().getFullYear() };
}

export function App() {
  const [classes, setClasses] = useState<Class[]>([]);
  const [selection, setSelection] = useState<Selection>({ kind: "none" });
  const [draft, setDraft] = useState<ClassInput>(emptyDraft);
  const [saved, setSaved] = useState<ClassInput>(emptyDraft);
  const [students, setStudents] = useState<Student[]>([]);
  const [studentText, setStudentText] = useState("");
  const [saving, setSaving] = useState(false);

  const [config, setConfig] = useState<Config | null>(null);
  const [info, setInfo] = useState<AppInfo | null>(null);
  const [dialog, setDialog] = useState<Dialog>(null);

  // Zwei Sorten Fehler, wie überall im Kit: mit Feldern an die Felder,
  // ohne Felder als Meldung oben.
  const [fieldError, setFieldError] = useState<ApiError | null>(null);
  const [banner, setBanner] = useState<string | null>(null);

  const dirty = draft.name !== saved.name || draft.year !== saved.year;

  /** Ein fehlgeschlagener Befehl: ans Feld oder nach oben. */
  const fail = useCallback((raw: unknown) => {
    const err = ApiError.from(raw);
    if (err.isValidation) {
      setFieldError(err);
    } else {
      setBanner(err.message);
    }
  }, []);

  const reload = useCallback(async () => {
    try {
      setClasses(await api.classes.list());
    } catch (raw) {
      fail(raw);
    }
  }, [fail]);

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

  // --- Klassen -------------------------------------------------------------

  function open(next: Selection, input: ClassInput, nextStudents: Student[] = []) {
    setSelection(next);
    setDraft(input);
    setSaved(input);
    setStudents(nextStudents);
    setStudentText("");
    setFieldError(null);
  }

  async function save(): Promise<boolean> {
    setSaving(true);
    try {
      const input = { name: draft.name, year: draft.year };
      const cls =
        selection.kind === "class"
          ? await api.classes.update(selection.id, input)
          : await api.classes.create(input);
      // Rust hat aufgeräumt (Leerzeichen am Rand) — das Ergebnis übernehmen.
      const list =
        selection.kind === "class" && selection.id === cls.id
          ? students
          : await api.students.list(cls.id);
      open({ kind: "class", id: cls.id }, { name: cls.name, year: cls.year }, list);
      await reload();
      return true;
    } catch (raw) {
      fail(raw);
      return false;
    } finally {
      setSaving(false);
    }
  }

  /** Wechseln speichert. Scheitert das Speichern, bleibt die Klasse offen. */
  async function leaveCurrent(): Promise<boolean> {
    if (selection.kind === "none" || !dirty) return true;
    return save();
  }

  async function select(id: string) {
    if (selection.kind === "class" && selection.id === id) return;
    if (!(await leaveCurrent())) return;
    const cls = classes.find((c) => c.id === id);
    if (!cls) return;
    try {
      open({ kind: "class", id }, { name: cls.name, year: cls.year }, await api.students.list(id));
    } catch (raw) {
      fail(raw);
    }
  }

  async function createNew() {
    if (!(await leaveCurrent())) return;
    open({ kind: "new" }, emptyDraft());
  }

  async function removeClass() {
    setDialog(null);
    if (selection.kind !== "class") return;
    try {
      await api.classes.remove(selection.id);
      open({ kind: "none" }, emptyDraft());
      await reload();
    } catch (raw) {
      fail(raw);
    }
  }

  // --- Schüler -------------------------------------------------------------

  async function addStudents() {
    if (selection.kind !== "class") return;
    try {
      await api.students.add(selection.id, studentText);
      setStudents(await api.students.list(selection.id));
      setStudentText("");
      setFieldError(null);
      await reload();
    } catch (raw) {
      fail(raw);
    }
  }

  async function removeStudent(student: Student) {
    setDialog(null);
    if (selection.kind !== "class") return;
    try {
      await api.students.remove(student.id);
      setStudents(await api.students.list(selection.id));
      await reload();
    } catch (raw) {
      fail(raw);
    }
  }

  // --- Einstellungen -------------------------------------------------------

  async function saveConfig(next: Config) {
    try {
      setConfig(await api.config.set(next));
      setFieldError(null);
      setDialog(null);
    } catch (raw) {
      fail(raw);
    }
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
  const selected = selection.kind === "class" ? classes.find((c) => c.id === selection.id) : null;

  return (
    <div className="app">
      <aside className="sidebar">
        <div className="brand">{config?.app_name ?? " "}</div>
        <ClassList
          classes={classes}
          selectedId={selection.kind === "class" ? selection.id : null}
          onSelect={(id) => void select(id)}
          onCreate={() => void createNew()}
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
          <EmptyState hasClasses={classes.length > 0} onCreate={() => void createNew()} />
        ) : (
          <ClassEditor
            draft={draft}
            isNew={selection.kind === "new"}
            dirty={dirty}
            saving={saving}
            students={students}
            studentText={studentText}
            fieldMessage={dialog ? () => undefined : fieldMessage}
            onChange={setDraft}
            onSave={() => void save()}
            onDelete={() => setDialog("delete-class")}
            onStudentTextChange={setStudentText}
            onAddStudents={() => void addStudents()}
            onDeleteStudent={(student) => setDialog({ student })}
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

      {dialog === "delete-class" && (
        <ConfirmDialog
          title="Klasse löschen"
          message={`„${classLabel(saved)}" wird mit ${selected?.student_count ?? 0} Schülern endgültig gelöscht.`}
          confirmLabel="Endgültig löschen"
          onConfirm={() => void removeClass()}
          onCancel={() => setDialog(null)}
        />
      )}

      {dialog !== null && typeof dialog === "object" && (
        <ConfirmDialog
          title="Schüler entfernen"
          message={`„${dialog.student.last_name}, ${dialog.student.first_name}" wird endgültig entfernt.`}
          confirmLabel="Endgültig entfernen"
          onConfirm={() => void removeStudent(dialog.student)}
          onCancel={() => setDialog(null)}
        />
      )}
    </div>
  );
}

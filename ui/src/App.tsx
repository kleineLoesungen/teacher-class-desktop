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
import type {
  AppInfo,
  Class,
  ClassInput,
  Config,
  Exam,
  ScoreInput,
  Student,
  StudentResult,
} from "./bindings";
import { ClassEditor } from "./components/ClassEditor";
import { ClassList } from "./components/ClassList";
import { ConfirmDialog } from "./components/ConfirmDialog";
import { EmptyState } from "./components/EmptyState";
import { ExamEditor } from "./components/ExamEditor";
import { ExamList } from "./components/ExamList";
import { GradingView } from "./components/GradingView";
import { SettingsDialog } from "./components/SettingsDialog";
import { draftFromExam, type ExamDraft, emptyExamDraft, sameDraft, toExamInput } from "./lib/exam";
import { classLabel } from "./lib/format";
import { applyTheme } from "./lib/theme";

type Selection = { kind: "none" } | { kind: "new" } | { kind: "class"; id: string };
type ExamSelection = { kind: "none" } | { kind: "new" } | { kind: "exam"; id: string };

type View = "classes" | "exams";

type Dialog = "settings" | "delete-class" | "delete-exam" | { student: Student } | null;

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

  const [view, setView] = useState<View>("classes");
  const [exams, setExams] = useState<Exam[]>([]);
  const [examSelection, setExamSelection] = useState<ExamSelection>({ kind: "none" });
  const [examDraft, setExamDraft] = useState<ExamDraft>(emptyExamDraft);
  const [examSaved, setExamSaved] = useState<ExamDraft>(emptyExamDraft);
  const [notice, setNotice] = useState<string | null>(null);
  // Bewertung: ist sie offen, ersetzt sie den Prüfungseditor.
  const [grading, setGrading] = useState<{ classId: string; studentId: string | null } | null>(
    null,
  );
  const [results, setResults] = useState<StudentResult[]>([]);

  const [config, setConfig] = useState<Config | null>(null);
  const [info, setInfo] = useState<AppInfo | null>(null);
  const [dialog, setDialog] = useState<Dialog>(null);

  // Zwei Sorten Fehler, wie überall in der App: mit Feldern an die Felder,
  // ohne Felder als Meldung oben.
  const [fieldError, setFieldError] = useState<ApiError | null>(null);
  const [banner, setBanner] = useState<string | null>(null);

  const dirty = draft.name !== saved.name || draft.year !== saved.year;
  const examDirty = !sameDraft(examDraft, examSaved);

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

  const reloadExams = useCallback(async () => {
    try {
      setExams(await api.exams.list());
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
      await Promise.all([reload(), reloadExams()]);
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
      // Prüfungen verlieren die gelöschte Klasse.
      await Promise.all([reload(), reloadExams()]);
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

  // --- Prüfungen -----------------------------------------------------------

  function openExam(next: ExamSelection, input: ExamDraft) {
    setExamSelection(next);
    setExamDraft(input);
    setExamSaved(input);
    setFieldError(null);
    setNotice(null);
    setGrading(null);
  }

  /** Die gespeicherte Prüfung, oder `null`, wenn Rust sie abgelehnt hat. */
  async function saveExam(): Promise<Exam | null> {
    setSaving(true);
    try {
      const input = toExamInput(examDraft);
      const exam =
        examSelection.kind === "exam"
          ? await api.exams.update(examSelection.id, input)
          : await api.exams.create(input);
      openExam({ kind: "exam", id: exam.id }, draftFromExam(exam));
      await reloadExams();
      return exam;
    } catch (raw) {
      fail(raw);
      return null;
    } finally {
      setSaving(false);
    }
  }

  async function leaveCurrentExam(): Promise<boolean> {
    if (examSelection.kind === "none" || !examDirty) return true;
    return (await saveExam()) !== null;
  }

  // --- Bewertung -----------------------------------------------------------

  /** Öffnet die Bewertung einer Klasse der Prüfung; speichert zuerst. */
  async function openGrading(classId?: string) {
    if (examSelection.kind !== "exam") return;
    let exam = exams.find((e) => e.id === examSelection.id) ?? null;
    if (examDirty) exam = await saveExam();
    const target = classId ?? exam?.class_ids[0];
    if (!exam || !target) return;
    try {
      const list = await api.exams.results(exam.id, target);
      setResults(list);
      setGrading({ classId: target, studentId: list[0]?.student.id ?? null });
      setFieldError(null);
      setNotice(null);
    } catch (raw) {
      fail(raw);
    }
  }

  async function scoreStudent(input: ScoreInput) {
    if (examSelection.kind !== "exam") return;
    try {
      const updated = await api.exams.score(examSelection.id, input);
      setResults((all) => all.map((r) => (r.student.id === updated.student.id ? updated : r)));
      setFieldError(null);
    } catch (raw) {
      fail(raw);
    }
  }

  async function selectExam(id: string) {
    if (examSelection.kind === "exam" && examSelection.id === id) return;
    if (!(await leaveCurrentExam())) return;
    const exam = exams.find((e) => e.id === id);
    if (exam) openExam({ kind: "exam", id }, draftFromExam(exam));
  }

  async function createNewExam() {
    if (!(await leaveCurrentExam())) return;
    openExam({ kind: "new" }, emptyExamDraft());
  }

  async function removeExam() {
    setDialog(null);
    if (examSelection.kind !== "exam") return;
    try {
      await api.exams.remove(examSelection.id);
      openExam({ kind: "none" }, emptyExamDraft());
      await reloadExams();
    } catch (raw) {
      fail(raw);
    }
  }

  /** Speichert zuerst, dann fragt der Dialog nach dem Ziel. */
  async function exportExam() {
    if (examSelection.kind !== "exam") return;
    if (examDirty && !(await saveExam())) return;
    const exam = exams.find((e) => e.id === examSelection.id);
    try {
      const path = await api.pickExportPath(`${exam?.name ?? examDraft.name}.xlsx`);
      if (path === null) return;
      await api.exams.export(examSelection.id, path);
      setBanner(null);
      setNotice(`Gespeichert: ${path}`);
    } catch (raw) {
      setNotice(null);
      fail(raw);
    }
  }

  async function switchView(next: View) {
    if (next === view) return;
    if (!(view === "classes" ? await leaveCurrent() : await leaveCurrentExam())) return;
    setGrading(null);
    setFieldError(null);
    setNotice(null);
    setView(next);
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
      const inClasses = view === "classes";
      if (e.key === "s" && (inClasses ? selection.kind : examSelection.kind) !== "none") {
        e.preventDefault();
        void (inClasses ? save() : saveExam());
      } else if (e.key === "n") {
        e.preventDefault();
        void (inClasses ? createNew() : createNewExam());
      }
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  });

  // --- Zeichnen ------------------------------------------------------------

  const fieldMessage = (field: string) => fieldError?.fieldMessage(field);
  const selected = selection.kind === "class" ? classes.find((c) => c.id === selection.id) : null;
  const gradingExam =
    examSelection.kind === "exam" ? (exams.find((e) => e.id === examSelection.id) ?? null) : null;

  return (
    <div className="app">
      <aside className="sidebar">
        <div className="brand">{config?.app_name ?? " "}</div>
        <div className="view-tabs">
          <button
            type="button"
            className="ghost"
            aria-pressed={view === "classes"}
            onClick={() => void switchView("classes")}
          >
            Klassen
          </button>
          <button
            type="button"
            className="ghost"
            aria-pressed={view === "exams"}
            onClick={() => void switchView("exams")}
          >
            Prüfungen
          </button>
        </div>
        {view === "classes" ? (
          <ClassList
            classes={classes}
            selectedId={selection.kind === "class" ? selection.id : null}
            onSelect={(id) => void select(id)}
            onCreate={() => void createNew()}
          />
        ) : (
          <ExamList
            exams={exams}
            selectedId={examSelection.kind === "exam" ? examSelection.id : null}
            onSelect={(id) => void selectExam(id)}
            onCreate={() => void createNewExam()}
          />
        )}
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

        {notice && (
          <div className="notice" role="status">
            <span>{notice}</span>
            <button type="button" className="ghost" onClick={() => setNotice(null)}>
              Schließen
            </button>
          </div>
        )}

        {view === "exams" ? (
          examSelection.kind === "none" ? (
            <EmptyState
              title={exams.length > 0 ? "Keine Prüfung ausgewählt" : "Noch keine Prüfungen"}
              text="Eine Prüfung hat ein Schema aus Gruppen und Punkten und eine Bewertungsmatrix. Bewertet wird in der App oder in der Excel-Datei; mit Begründungen gibt es im Export eine Bewertungskachel je Schüler."
              action="Neue Prüfung"
              onCreate={() => void createNewExam()}
            />
          ) : grading !== null && examSelection.kind === "exam" && gradingExam ? (
            <GradingView
              exam={gradingExam}
              classes={classes.filter((c) => gradingExam.class_ids.includes(c.id))}
              classId={grading.classId}
              results={results}
              studentId={grading.studentId}
              messages={(field) => fieldError?.fieldMessages(field) ?? []}
              onSelectClass={(id) => void openGrading(id)}
              onSelectStudent={(id) => {
                setFieldError(null);
                setGrading({ ...grading, studentId: id });
              }}
              onScore={(input) => void scoreStudent(input)}
              onBack={() => {
                setGrading(null);
                setFieldError(null);
              }}
              onExport={() => void exportExam()}
            />
          ) : (
            <ExamEditor
              draft={examDraft}
              exam={
                examSelection.kind === "exam"
                  ? (exams.find((e) => e.id === examSelection.id) ?? null)
                  : null
              }
              classes={classes}
              isNew={examSelection.kind === "new"}
              dirty={examDirty}
              saving={saving}
              fieldMessages={dialog ? () => [] : (field) => fieldError?.fieldMessages(field) ?? []}
              onChange={setExamDraft}
              onSave={() => void saveExam()}
              onDelete={() => setDialog("delete-exam")}
              onExport={() => void exportExam()}
              onGrade={() => void openGrading()}
            />
          )
        ) : selection.kind === "none" ? (
          <EmptyState
            title={classes.length > 0 ? "Keine Klasse ausgewählt" : "Willkommen"}
            text="Lege Klassen mit ihren Schülern an. Darauf bauen die Prüfungen und der Excel-Export auf."
            action="Neue Klasse"
            onCreate={() => void createNew()}
          />
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

      {dialog === "delete-exam" && (
        <ConfirmDialog
          title="Prüfung löschen"
          message={`„${examSaved.name}" wird mit Schema und Bewertungsmatrix endgültig gelöscht.`}
          confirmLabel="Endgültig löschen"
          onConfirm={() => void removeExam()}
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

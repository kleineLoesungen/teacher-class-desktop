// Eine Klasse bearbeiten und ihre Schüler verwalten.
//
// Zeichnet nur. Entwurf und Texteingabe gehören App.tsx; hier werden sie
// angezeigt und jede Änderung nach oben gemeldet. Ob ein Name gültig ist,
// entscheidet Rust — die Meldungen kommen von dort (`fieldMessage`).

import type { ClassInput, Student } from "../bindings";
import { FieldMessage } from "./FieldMessage";

type Props = {
  draft: ClassInput;
  isNew: boolean;
  dirty: boolean;
  saving: boolean;
  students: Student[];
  /** Eingabefeld „Schüler hinzufügen": eine Zeile je Schüler. */
  studentText: string;
  fieldMessage: (field: string) => string | undefined;
  onChange: (draft: ClassInput) => void;
  onSave: () => void;
  onDelete: () => void;
  onStudentTextChange: (text: string) => void;
  onAddStudents: () => void;
  onDeleteStudent: (student: Student) => void;
};

export function ClassEditor(props: Props) {
  const { draft, isNew, dirty, saving, students, studentText, fieldMessage } = props;
  const nameError = fieldMessage("name");
  const yearError = fieldMessage("year");
  const studentsError = fieldMessage("students");

  return (
    <section className="editor" aria-label="Klasse">
      <header className="editor-head">
        <div className="field grow">
          <input
            className="title-input"
            aria-label="Klassenname"
            placeholder="Klassenname, z. B. 7a"
            value={draft.name}
            aria-invalid={nameError ? true : undefined}
            aria-describedby={nameError ? "name-error" : undefined}
            onChange={(e) => props.onChange({ ...draft, name: e.target.value })}
          />
          <FieldMessage id="name-error" message={nameError} />
        </div>

        <div className="field year-field">
          <input
            className="title-input"
            aria-label="Jahr"
            placeholder="Jahr"
            inputMode="numeric"
            value={draft.year === 0 ? "" : String(draft.year)}
            aria-invalid={yearError ? true : undefined}
            aria-describedby={yearError ? "year-error" : undefined}
            onChange={(e) => {
              const digits = e.target.value.replace(/\D/g, "").slice(0, 4);
              props.onChange({ ...draft, year: digits === "" ? 0 : Number(digits) });
            }}
          />
          <FieldMessage id="year-error" message={yearError} />
        </div>

        {!isNew && (
          <button type="button" className="ghost" onClick={props.onDelete}>
            Klasse löschen
          </button>
        )}
        <button
          type="button"
          className="primary"
          disabled={saving || (!dirty && !isNew)}
          onClick={props.onSave}
        >
          {saving ? "Speichert …" : "Speichern"}
        </button>
      </header>

      {isNew ? (
        <p className="muted class-body">Nach dem Speichern können Schüler hinzugefügt werden.</p>
      ) : (
        <div className="class-body">
          <h2>Schüler</h2>
          {students.length === 0 ? (
            <p className="muted">Noch keine Schüler in dieser Klasse.</p>
          ) : (
            <ul className="student-list" aria-label="Schüler der Klasse">
              {students.map((s) => (
                <li key={s.id} className="student-row">
                  <span className="grow">
                    {s.last_name}, {s.first_name}
                  </span>
                  <button
                    type="button"
                    className="ghost"
                    aria-label={`${s.last_name}, ${s.first_name} entfernen`}
                    onClick={() => props.onDeleteStudent(s)}
                  >
                    Entfernen
                  </button>
                </li>
              ))}
            </ul>
          )}

          <div className="field student-add">
            <label htmlFor="student-text">Schüler hinzufügen</label>
            <textarea
              id="student-text"
              rows={6}
              placeholder={"Nachname, Vorname\nNachname, Vorname"}
              value={studentText}
              spellCheck={false}
              aria-invalid={studentsError ? true : undefined}
              aria-describedby={studentsError ? "students-error" : "students-hint"}
              onChange={(e) => props.onStudentTextChange(e.target.value)}
            />
            <p id="students-hint" className="muted hint">
              Eine Zeile je Schüler: Nachname, Vorname
            </p>
            <FieldMessage id="students-error" message={studentsError} />
            <div className="actions">
              <button
                type="button"
                className="primary"
                disabled={studentText.trim() === ""}
                onClick={props.onAddStudents}
              >
                Hinzufügen
              </button>
            </div>
          </div>
        </div>
      )}
    </section>
  );
}

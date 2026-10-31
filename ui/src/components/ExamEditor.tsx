// Eine Prüfung bearbeiten: Klassen, Prüfungsschema und Bewertungsmatrix.
//
// Zeichnet nur. Der Entwurf gehört App.tsx. Ob etwas gültig ist, entscheidet
// Rust — die Meldungen kommen von dort. Das gilt auch für die berechneten
// Werte: Höchstpunktzahlen und das „bis“ der Noten rechnet Rust. Sie kommen
// als Vorschau (`preview`) zum Entwurf und ziehen beim Tippen nach, ohne dass
// gespeichert werden muss. Fehlt ein Wert (noch nicht gerechnet), steht ein Strich.

import type { Class, Preview } from "../bindings";
import { type ExamDraft, emptyItemDraft, formatPoints } from "../lib/exam";
import { classLabel } from "../lib/format";
import { FieldMessage } from "./FieldMessage";

type Props = {
  draft: ExamDraft;
  /** Von Rust berechnete Werte des Entwurfs; `null`, solange keine da sind. */
  preview: Preview | null;
  classes: Class[];
  isNew: boolean;
  dirty: boolean;
  saving: boolean;
  fieldMessages: (field: string) => string[];
  onChange: (draft: ExamDraft) => void;
  onSave: () => void;
  onDelete: () => void;
  onExport: () => void;
  onGrade: () => void;
};

function Messages({ id, messages }: { id: string; messages: string[] }) {
  if (messages.length === 0) return null;
  return (
    <div id={id} role="alert">
      {messages.map((m) => (
        <p key={m} className="field-message">
          {m}
        </p>
      ))}
    </div>
  );
}

export function ExamEditor(props: Props) {
  const { draft, preview, classes, isNew, dirty, saving, fieldMessages, onChange } = props;
  const dash = "–";

  const nameError = fieldMessages("name")[0];
  const setGroup = (gi: number, group: ExamDraft["groups"][number]) =>
    onChange({ ...draft, groups: draft.groups.map((g, i) => (i === gi ? group : g)) });
  const setGrade = (ni: number, grade: ExamDraft["grades"][number]) =>
    onChange({ ...draft, grades: draft.grades.map((g, i) => (i === ni ? grade : g)) });
  const toggleClass = (id: string, on: boolean) =>
    onChange({
      ...draft,
      class_ids: on ? [...draft.class_ids, id] : draft.class_ids.filter((c) => c !== id),
    });

  return (
    <section className="editor" aria-label="Prüfung">
      <header className="editor-head">
        <div className="field grow">
          <input
            className="title-input"
            aria-label="Name der Prüfung"
            placeholder="Name der Prüfung, z. B. Klassenarbeit 1"
            value={draft.name}
            aria-invalid={nameError ? true : undefined}
            aria-describedby={nameError ? "exam-name-error" : undefined}
            onChange={(e) => onChange({ ...draft, name: e.target.value })}
          />
          <FieldMessage id="exam-name-error" message={nameError} />
        </div>
        {!isNew && (
          <>
            <button type="button" className="ghost" onClick={props.onDelete}>
              Prüfung löschen
            </button>
            <button type="button" onClick={props.onGrade}>
              Bewerten
            </button>
            <button type="button" onClick={props.onExport}>
              Als Excel exportieren
            </button>
          </>
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

      <div className="class-body exam-body">
        <h2>Klassen</h2>
        {classes.length === 0 ? (
          <p className="muted">Lege zuerst Klassen an.</p>
        ) : (
          <fieldset className="check-list" aria-label="Klassen der Prüfung">
            {classes.map((c) => (
              <label key={c.id} className="check">
                <input
                  type="checkbox"
                  checked={draft.class_ids.includes(c.id)}
                  onChange={(e) => toggleClass(c.id, e.target.checked)}
                />
                {classLabel(c)}
              </label>
            ))}
          </fieldset>
        )}
        <Messages id="classes-error" messages={fieldMessages("classes")} />

        <h2>Prüfungsschema</h2>
        <p className="muted hint">
          Gruppen mit Punkten. Ganze oder halbe Punkte, z. B. 2,5. Die Höchstpunktzahlen von Gruppe
          und Prüfung werden berechnet.
        </p>
        {draft.groups.map((group, gi) => {
          const g = gi + 1;
          const sum = preview?.group_max_points[gi];
          return (
            // biome-ignore lint/suspicious/noArrayIndexKey: Gruppen haben keine Kennung, nur eine Stelle
            <div key={gi} className="group-card">
              <div className="row">
                <input
                  className="grow"
                  aria-label={`Gruppe ${g} Name`}
                  placeholder="Name der Gruppe, z. B. Aufgabe 1"
                  value={group.name}
                  onChange={(e) => setGroup(gi, { ...group, name: e.target.value })}
                />
                <span className="muted points-sum">
                  max. {sum !== undefined ? formatPoints(sum) : dash}
                </span>
                <button
                  type="button"
                  className="ghost"
                  aria-label={`Gruppe ${g} entfernen`}
                  onClick={() =>
                    onChange({ ...draft, groups: draft.groups.filter((_, i) => i !== gi) })
                  }
                >
                  Entfernen
                </button>
              </div>
              {group.items.map((item, ii) => {
                const n = ii + 1;
                return (
                  // biome-ignore lint/suspicious/noArrayIndexKey: Punkte haben keine Kennung, nur eine Stelle
                  <div key={ii} className="row item-row">
                    <input
                      className="item-name"
                      aria-label={`Gruppe ${g} Punkt ${n} Name`}
                      placeholder="Punkt, z. B. 1a"
                      value={item.name}
                      onChange={(e) =>
                        setGroup(gi, {
                          ...group,
                          items: group.items.map((x, i) =>
                            i === ii ? { ...x, name: e.target.value } : x,
                          ),
                        })
                      }
                    />
                    <input
                      className="grow"
                      aria-label={`Gruppe ${g} Punkt ${n} Beschreibung`}
                      placeholder="Beschreibung (optional)"
                      value={item.description}
                      onChange={(e) =>
                        setGroup(gi, {
                          ...group,
                          items: group.items.map((x, i) =>
                            i === ii ? { ...x, description: e.target.value } : x,
                          ),
                        })
                      }
                    />
                    <input
                      className="points-input"
                      inputMode="decimal"
                      aria-label={`Gruppe ${g} Punkt ${n} Höchstpunkte`}
                      placeholder="Punkte"
                      value={item.max}
                      onChange={(e) =>
                        setGroup(gi, {
                          ...group,
                          items: group.items.map((x, i) =>
                            i === ii ? { ...x, max: e.target.value } : x,
                          ),
                        })
                      }
                    />
                    <button
                      type="button"
                      className="ghost"
                      aria-label={`Gruppe ${g} Punkt ${n} entfernen`}
                      onClick={() =>
                        setGroup(gi, { ...group, items: group.items.filter((_, i) => i !== ii) })
                      }
                    >
                      Entfernen
                    </button>
                  </div>
                );
              })}
              <button
                type="button"
                className="ghost"
                aria-label={`Punkt zu Gruppe ${g} hinzufügen`}
                onClick={() =>
                  setGroup(gi, { ...group, items: [...group.items, emptyItemDraft()] })
                }
              >
                + Punkt
              </button>
            </div>
          );
        })}
        <div className="row">
          <button
            type="button"
            onClick={() =>
              onChange({
                ...draft,
                groups: [...draft.groups, { name: "", items: [emptyItemDraft()] }],
              })
            }
          >
            Gruppe hinzufügen
          </button>
          <span className="muted">
            Gesamtpunkte: {preview ? formatPoints(preview.max_points) : dash}
          </span>
        </div>
        <Messages id="groups-error" messages={fieldMessages("groups")} />

        <h2>Begründungen</h2>
        <label className="check">
          <input
            type="checkbox"
            checked={draft.with_notes}
            onChange={(e) => onChange({ ...draft, with_notes: e.target.checked })}
          />
          Zu jedem Prüfkriterium eine Begründung pflegen
        </label>
        <p className="muted hint">
          Mit Begründungen enthält der Excel-Export statt einer Matrix eine Bewertungskachel je
          Schüler, eine Seite pro Schüler zum Drucken.
        </p>

        <h2>Bewertungsmatrix</h2>
        <p className="muted hint">
          Je Note nur „ab“ (Punkte). „Bis“ wird berechnet; die niedrigste Note beginnt bei 0.
        </p>
        <table className="grade-table">
          <thead>
            <tr>
              <th scope="col">Note</th>
              <th scope="col">ab</th>
              <th scope="col">bis</th>
              <th scope="col">
                <span className="visually-hidden">Aktion</span>
              </th>
            </tr>
          </thead>
          <tbody>
            {draft.grades.map((grade, ni) => {
              const n = ni + 1;
              const to = preview?.grade_to_points[ni];
              return (
                // biome-ignore lint/suspicious/noArrayIndexKey: Noten haben keine Kennung, nur eine Stelle
                <tr key={ni}>
                  <td>
                    <input
                      aria-label={`Note ${n} Name`}
                      value={grade.name}
                      onChange={(e) => setGrade(ni, { ...grade, name: e.target.value })}
                    />
                  </td>
                  <td>
                    <input
                      className="points-input"
                      inputMode="decimal"
                      aria-label={`Note ${n} ab`}
                      value={grade.from}
                      onChange={(e) => setGrade(ni, { ...grade, from: e.target.value })}
                    />
                  </td>
                  <td className="muted">{to != null ? formatPoints(to) : dash}</td>
                  <td>
                    <button
                      type="button"
                      className="ghost"
                      aria-label={`Note ${n} entfernen`}
                      onClick={() =>
                        onChange({ ...draft, grades: draft.grades.filter((_, i) => i !== ni) })
                      }
                    >
                      Entfernen
                    </button>
                  </td>
                </tr>
              );
            })}
          </tbody>
        </table>
        <div className="row">
          <button
            type="button"
            onClick={() =>
              onChange({ ...draft, grades: [...draft.grades, { name: "", from: "" }] })
            }
          >
            Note hinzufügen
          </button>
        </div>
        <Messages id="grades-error" messages={fieldMessages("grades")} />

        {!isNew && (
          <p className="muted hint">
            Der Excel-Export enthält das Blatt „Bewertung“ und ein Blatt je Klasse, jeweils mit den
            in der App eingetragenen Punkten{draft.with_notes ? " und Begründungen" : ""}.
          </p>
        )}
      </div>
    </section>
  );
}

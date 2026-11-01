// Eine Prüfung bewerten: Punkte und (falls die Prüfung es vorsieht)
// Begründungen je Schüler und Prüfkriterium.
//
// Zeichnet nur. Zustand und Befehle gehören App.tsx. Summen, Gesamtpunkte und
// Note kommen fertig berechnet aus Rust (`StudentResult`) — hier wird nicht
// gerechnet. Gespeichert wird, sobald ein Feld verlassen wird; ob die Eingabe
// gültig ist, entscheidet Rust, die Meldung erscheint über dem Schüler.

import { useRef, useState } from "react";
import type {
  Class,
  Exam,
  GradeKey,
  RemarkInput,
  ScoreInput,
  StudentResult,
  SuggestInput,
  SuggestKind,
  TemplateInfo,
} from "../bindings";
import { formatAverage, formatPoints, parsePoints } from "../lib/exam";
import { classLabel } from "../lib/format";

type Props = {
  exam: Exam;
  /** Die Klassen der Prüfung. */
  classes: Class[];
  classId: string | null;
  results: StudentResult[];
  /** Anzahl je Note und Durchschnitt der Klasse; von Rust berechnet. */
  gradeKey: GradeKey | null;
  studentId: string | null;
  /** Meldungen von Rust zu einem Feld (`points`, `note`, `item`). */
  messages: (field: string) => string[];
  onSelectClass: (id: string) => void;
  onSelectStudent: (id: string) => void;
  onScore: (input: ScoreInput) => void;
  /** Setzt den Abschluss-Satz; leer löscht ihn. */
  onRemark: (input: RemarkInput) => void;
  /** Texte anderer Schüler derselben Prüfung, die zur Eingabe passen. */
  onSuggest: (input: SuggestInput) => Promise<string[]>;
  onBack: () => void;
  onExport: () => void;
  /** Die gewählte Klasse als PDF, gesetzt aus der gewählten Druckvorlage. */
  onExportPdf: () => void;
  /** Die Druckvorlagen neben der Datenbank; leer, bis Rust sie geliefert hat. */
  templates: TemplateInfo[];
  /** Der Ordner, in dem man eigene Vorlagen ablegt. */
  templateDir: string;
  /** Dateiname der gewählten Vorlage. */
  template: string;
  onSelectTemplate: (file: string) => void;
};

/** Der Name einer Vorlage, wie er in der Auswahl steht. */
function templateLabel(t: TemplateInfo): string {
  return t.standard ? "Standard" : t.name;
}

type CellDraft = { points: string; note: string };

/** Der Text, wie er im Eingabefeld steht. */
function draftOf(result: StudentResult, gi: number, ii: number): CellDraft {
  const score = result.groups[gi]?.scores[ii];
  return {
    points: score?.points == null ? "" : formatPoints(score.points),
    note: score?.note ?? "",
  };
}

/**
 * Ein Textfeld mit Vorschlägen aus den Texten anderer Schüler. Die Vorschläge
 * wählt Rust aus (`onSuggest`); hier wird nur angezeigt und eingefügt. Gespeichert
 * wird, sobald der Fokus das Feld samt Vorschlägen verlässt oder ein Vorschlag
 * gewählt wird.
 */
function SuggestField({
  kind,
  studentId,
  label,
  placeholder,
  className,
  value,
  onChange,
  onCommit,
  onSuggest,
}: {
  kind: SuggestKind;
  studentId: string;
  label: string;
  placeholder: string;
  className?: string;
  value: string;
  onChange: (text: string) => void;
  onCommit: (text: string) => void;
  onSuggest: (input: SuggestInput) => Promise<string[]>;
}) {
  const [list, setList] = useState<string[]>([]);
  // Nur die jüngste Antwort zählt, sonst überholt eine späte alte die neue.
  const latest = useRef(0);

  function ask(query: string) {
    const n = ++latest.current;
    void onSuggest({ kind, query, exclude_student_id: studentId }).then((texts) => {
      if (n === latest.current) setList(texts);
    });
  }

  return (
    <fieldset
      className="suggest"
      aria-label={`${label} mit Vorschlägen`}
      onBlur={(e) => {
        if (e.currentTarget.contains(e.relatedTarget as Node | null)) return;
        latest.current++;
        setList([]);
        onCommit(value);
      }}
    >
      <textarea
        className={className}
        rows={2}
        aria-label={label}
        placeholder={placeholder}
        value={value}
        onFocus={() => ask(value)}
        onChange={(e) => {
          onChange(e.target.value);
          ask(e.target.value);
        }}
      />
      {list.length > 0 && (
        <ul className="suggestions" aria-label={`Vorschläge ${label}`}>
          {list.map((text) => (
            <li key={text}>
              <button
                type="button"
                className="ghost"
                onClick={() => {
                  latest.current++;
                  setList([]);
                  onChange(text);
                  onCommit(text);
                }}
              >
                {text}
              </button>
            </li>
          ))}
        </ul>
      )}
    </fieldset>
  );
}

/** Schmaler Balken; die Zahlen stehen daneben, deshalb für Bildschirmleser stumm.
 *  Der Anteil ist reine Darstellung, keine Fachregel. */
function Progress({ value, max }: { value: number; max: number }) {
  const share = max > 0 ? Math.min(100, Math.max(0, (value / max) * 100)) : 0;
  return (
    <span className="progress" aria-hidden="true">
      <span style={{ width: `${share}%` }} />
    </span>
  );
}

function StudentTile({
  exam,
  result,
  messages,
  onScore,
  onRemark,
  onSuggest,
}: {
  exam: Exam;
  result: StudentResult;
  messages: (field: string) => string[];
  onScore: (input: ScoreInput) => void;
  onRemark: (input: RemarkInput) => void;
  onSuggest: (input: SuggestInput) => Promise<string[]>;
}) {
  const [remark, setRemark] = useState(result.remark);
  // Eigene Entwürfe je Feld, damit Tippen nicht auf die Antwort von Rust wartet.
  // Die Kachel wird je Schüler neu aufgebaut (`key`), nicht je Antwort — sonst
  // ginge beim Tabben der Fokus verloren.
  const [drafts, setDrafts] = useState<Record<string, CellDraft>>(() => {
    const initial: Record<string, CellDraft> = {};
    exam.groups.forEach((g, gi) => {
      g.items.forEach((_, ii) => {
        initial[`${gi}:${ii}`] = draftOf(result, gi, ii);
      });
    });
    return initial;
  });

  const draftFor = (gi: number, ii: number): CellDraft =>
    drafts[`${gi}:${ii}`] ?? { points: "", note: "" };

  function change(gi: number, ii: number, patch: Partial<CellDraft>) {
    setDrafts({ ...drafts, [`${gi}:${ii}`]: { ...draftFor(gi, ii), ...patch } });
  }

  /** Beim Verlassen eines Feldes: nur schicken, was sich geändert hat. */
  function commit(gi: number, ii: number, patch: Partial<CellDraft> = {}) {
    const draft = { ...draftFor(gi, ii), ...patch };
    const stored = draftOf(result, gi, ii);
    if (draft.points === stored.points && draft.note === stored.note) return;
    onScore({
      student_id: result.student.id,
      group: gi,
      item: ii,
      points: draft.points.trim() === "" ? null : parsePoints(draft.points),
      note: exam.with_notes ? draft.note : "",
    });
  }

  function commitRemark(text: string) {
    if (text.trim() === result.remark) return;
    onRemark({ student_id: result.student.id, text });
  }

  const errors = [
    ...messages("item"),
    ...messages("points"),
    ...messages("note"),
    ...messages("remark"),
  ];

  return (
    <section className="tile" aria-label={`Bewertung ${result.student.last_name}`}>
      <h3>
        {result.student.last_name}, {result.student.first_name}
      </h3>
      {errors.length > 0 && (
        <div role="alert">
          {errors.map((m) => (
            <p key={m} className="field-message">
              {m}
            </p>
          ))}
        </div>
      )}
      {exam.groups.map((group, gi) => {
        const g = gi + 1;
        return (
          // biome-ignore lint/suspicious/noArrayIndexKey: Gruppen haben keine Kennung, nur eine Stelle
          <div key={gi} className="group-card">
            <div className="row group-head">
              <strong className="grow">{group.name}</strong>
              <span className="muted points-sum">
                {formatPoints(result.groups[gi]?.points ?? 0)} / {formatPoints(group.max_points)}
              </span>
            </div>
            <Progress value={result.groups[gi]?.points ?? 0} max={group.max_points} />
            {group.items.map((item, ii) => {
              const n = ii + 1;
              const draft = draftFor(gi, ii);
              return (
                // biome-ignore lint/suspicious/noArrayIndexKey: Punkte haben keine Kennung, nur eine Stelle
                <div key={ii} className="criterion">
                  <div className="row">
                    <div className="grow">
                      <div>{item.name}</div>
                      {item.description && <div className="muted hint">{item.description}</div>}
                    </div>
                    <input
                      className="points-input"
                      inputMode="decimal"
                      aria-label={`Gruppe ${g} Punkt ${n} Punkte`}
                      placeholder="–"
                      value={draft.points}
                      onChange={(e) => change(gi, ii, { points: e.target.value })}
                      onBlur={() => commit(gi, ii)}
                    />
                    <span className="muted points-sum">/ {formatPoints(item.max_points)}</span>
                  </div>
                  {exam.with_notes && (
                    <SuggestField
                      kind="note"
                      studentId={result.student.id}
                      className="note-input"
                      label={`Gruppe ${g} Punkt ${n} Begründung`}
                      placeholder="Begründung / Hinweis"
                      value={draft.note}
                      onChange={(note) => change(gi, ii, { note })}
                      onCommit={(note) => commit(gi, ii, { note })}
                      onSuggest={onSuggest}
                    />
                  )}
                </div>
              );
            })}
          </div>
        );
      })}
      <div className="criterion">
        <SuggestField
          kind="remark"
          studentId={result.student.id}
          className="note-input"
          label="Abschluss-Satz"
          placeholder="Abschluss-Satz, z. B. „Tolle Leistung! Weiter so“"
          value={remark}
          onChange={setRemark}
          onCommit={commitRemark}
          onSuggest={onSuggest}
        />
      </div>
      <p className="tile-total">
        <strong>
          Gesamt: {formatPoints(result.total)} / {formatPoints(exam.max_points)}
        </strong>
        <strong className="grade-badge">Note: {result.grade ?? "–"}</strong>
      </p>
    </section>
  );
}

/** Der Notenschlüssel, eingeklappt: „1× Note 1, 3× Note 4 · Schnitt 2,60“. */
function GradeKeyView({ gradeKey }: { gradeKey: GradeKey }) {
  const given = gradeKey.counts.filter((c) => c.count > 0);
  return (
    <details className="grade-key">
      <summary>Notenschlüssel</summary>
      {given.length === 0 ? (
        <p className="muted">Noch keine Note vergeben.</p>
      ) : (
        <>
          <ul aria-label="Noten">
            {given.map((c) => (
              <li key={c.grade}>
                {c.count}× Note {c.grade}
              </li>
            ))}
          </ul>
          {gradeKey.average !== null && (
            <p>
              <strong>Schnitt {formatAverage(gradeKey.average)}</strong>
            </p>
          )}
        </>
      )}
    </details>
  );
}

export function GradingView(props: Props) {
  const { exam, classes, classId, results, studentId } = props;
  const current = results.find((r) => r.student.id === studentId) ?? null;
  const index = current ? results.indexOf(current) : -1;
  const next = index >= 0 ? results[index + 1] : undefined;

  return (
    <section className="editor" aria-label="Bewertung">
      <header className="editor-head">
        <h1 className="grow">Bewertung: {exam.name}</h1>
        <button type="button" disabled={classId === null} onClick={props.onExportPdf}>
          Klasse als PDF drucken
        </button>
        <button type="button" onClick={props.onExport}>
          Als Excel exportieren
        </button>
        <button type="button" className="ghost" onClick={props.onBack}>
          Zurück zur Prüfung
        </button>
      </header>

      <div className="class-body exam-body">
        <div className="row">
          <label htmlFor="grading-class">Klasse</label>
          <select
            id="grading-class"
            value={classId ?? ""}
            onChange={(e) => props.onSelectClass(e.target.value)}
          >
            {classes.map((c) => (
              <option key={c.id} value={c.id}>
                {classLabel(c)}
              </option>
            ))}
          </select>
        </div>

        <div className="row">
          <label htmlFor="grading-template">Druckvorlage</label>
          <select
            id="grading-template"
            value={props.template}
            onChange={(e) => props.onSelectTemplate(e.target.value)}
          >
            <optgroup label="Je Schüler (eine Seite je Schüler)">
              {props.templates
                .filter((t) => t.kind === "student")
                .map((t) => (
                  <option key={t.file} value={t.file}>
                    {templateLabel(t)}
                  </option>
                ))}
            </optgroup>
            <optgroup label="Je Klasse (eine Übersicht)">
              {props.templates
                .filter((t) => t.kind === "class")
                .map((t) => (
                  <option key={t.file} value={t.file}>
                    {templateLabel(t)}
                  </option>
                ))}
            </optgroup>
          </select>
        </div>
        {props.templateDir !== "" && (
          <p className="hint">
            Eigene Vorlagen (Markdown-Dateien) liegen in {props.templateDir}. Welche Platzhalter es
            gibt, steht dort in placeholder.md.
          </p>
        )}

        {props.gradeKey && results.length > 0 && <GradeKeyView gradeKey={props.gradeKey} />}

        {results.length === 0 ? (
          <p className="muted">Diese Klasse hat keine Schüler.</p>
        ) : (
          <div className="grading">
            <ul className="student-pick" aria-label="Schüler">
              {results.map((r) => (
                <li key={r.student.id}>
                  <button
                    type="button"
                    className="ghost"
                    aria-pressed={r.student.id === studentId}
                    onClick={() => props.onSelectStudent(r.student.id)}
                  >
                    <span>
                      {r.student.last_name}, {r.student.first_name}
                    </span>
                    <span className="muted">
                      {r.grade ? `${formatPoints(r.total)} · ${r.grade}` : ""}
                    </span>
                  </button>
                </li>
              ))}
            </ul>
            <div className="grow">
              {current ? (
                <>
                  <StudentTile
                    key={`${exam.id}:${current.student.id}`}
                    exam={exam}
                    result={current}
                    messages={props.messages}
                    onScore={props.onScore}
                    onRemark={props.onRemark}
                    onSuggest={props.onSuggest}
                  />
                  {next && (
                    <button type="button" onClick={() => props.onSelectStudent(next.student.id)}>
                      Nächster Schüler
                    </button>
                  )}
                </>
              ) : (
                <p className="muted">Wähle einen Schüler.</p>
              )}
            </div>
          </div>
        )}
      </div>
    </section>
  );
}

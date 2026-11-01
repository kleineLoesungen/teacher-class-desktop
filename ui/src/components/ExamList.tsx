// Die Liste der Prüfungen links. Zeichnet nur.

import type { Exam } from "../bindings";
import { formatPoints } from "../lib/exam";

type Props = {
  exams: Exam[];
  selectedId: string | null;
  onSelect: (id: string) => void;
  onCreate: () => void;
};

export function ExamList({ exams, selectedId, onSelect, onCreate }: Props) {
  const active = exams.filter((e) => !e.archived);
  const archived = exams.filter((e) => e.archived);
  // Ist eine archivierte Prüfung ausgewählt, bleibt der Bereich offen.
  const archivedSelected = archived.some((e) => e.id === selectedId);

  const item = (e: Exam) => (
    <li key={e.id}>
      <button
        type="button"
        className="class-item"
        aria-current={e.id === selectedId ? "true" : undefined}
        onClick={() => onSelect(e.id)}
      >
        <span className="class-title">{e.name}</span>
        <span className="class-excerpt">
          {e.class_ids.length === 1 ? "1 Klasse" : `${e.class_ids.length} Klassen`},{" "}
          {formatPoints(e.max_points)} Punkte
        </span>
      </button>
    </li>
  );

  return (
    <nav className="class-nav" aria-label="Prüfungen">
      <div className="sidebar-head">
        <button type="button" className="primary grow" onClick={onCreate}>
          Neue Prüfung
        </button>
      </div>

      {exams.length === 0 ? (
        <p className="muted sidebar-empty">Noch keine Prüfungen.</p>
      ) : (
        <div className="class-scroll">
          {active.length === 0 ? (
            <p className="muted sidebar-empty">Keine aktiven Prüfungen.</p>
          ) : (
            <ul className="class-list">{active.map(item)}</ul>
          )}
          {archived.length > 0 && (
            <details className="archive" open={archivedSelected || undefined}>
              <summary>Archiv ({archived.length})</summary>
              <ul className="class-list">{archived.map(item)}</ul>
            </details>
          )}
        </div>
      )}
    </nav>
  );
}

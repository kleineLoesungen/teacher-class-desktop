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
        <ul className="class-list">
          {exams.map((e) => (
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
          ))}
        </ul>
      )}
    </nav>
  );
}

// Die Liste links. Zeichnet nur, meldet Auswahl und Wünsche nach oben.

import type { Class } from "../bindings";
import { classLabel, studentCountLabel } from "../lib/format";

type Props = {
  classes: Class[];
  selectedId: string | null;
  onSelect: (id: string) => void;
  onCreate: () => void;
};

export function ClassList({ classes, selectedId, onSelect, onCreate }: Props) {
  return (
    <nav className="note-nav" aria-label="Klassen">
      <div className="sidebar-head">
        <button type="button" className="primary grow" onClick={onCreate}>
          Neue Klasse
        </button>
      </div>

      {classes.length === 0 ? (
        <p className="muted sidebar-empty">Noch keine Klassen.</p>
      ) : (
        <ul className="note-list">
          {classes.map((c) => (
            <li key={c.id}>
              <button
                type="button"
                className="note-item"
                aria-current={c.id === selectedId ? "true" : undefined}
                onClick={() => onSelect(c.id)}
              >
                <span className="note-title">{classLabel(c)}</span>
                <span className="note-excerpt">{studentCountLabel(c.student_count)}</span>
              </button>
            </li>
          ))}
        </ul>
      )}
    </nav>
  );
}

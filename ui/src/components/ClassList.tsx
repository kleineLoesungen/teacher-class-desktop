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
  const active = classes.filter((c) => !c.archived);
  const archived = classes.filter((c) => c.archived);
  // Ist eine archivierte Klasse ausgewählt, bleibt der Bereich offen.
  const archivedSelected = archived.some((c) => c.id === selectedId);

  const item = (c: Class) => (
    <li key={c.id}>
      <button
        type="button"
        className="class-item"
        aria-current={c.id === selectedId ? "true" : undefined}
        onClick={() => onSelect(c.id)}
      >
        <span className="class-title">{classLabel(c)}</span>
        <span className="class-excerpt">{studentCountLabel(c.student_count)}</span>
      </button>
    </li>
  );

  return (
    <nav className="class-nav" aria-label="Klassen">
      <div className="sidebar-head">
        <button type="button" className="primary grow" onClick={onCreate}>
          Neue Klasse
        </button>
      </div>

      {classes.length === 0 ? (
        <p className="muted sidebar-empty">Noch keine Klassen.</p>
      ) : (
        <div className="class-scroll">
          {active.length === 0 ? (
            <p className="muted sidebar-empty">Keine aktiven Klassen.</p>
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

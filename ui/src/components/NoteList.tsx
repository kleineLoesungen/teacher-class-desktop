// Die Liste links. Zeichnet nur, meldet Auswahl und Wünsche nach oben.

import type { Note, Order } from "../bindings";
import { excerpt, formatDateTime } from "../lib/format";

type Props = {
  notes: Note[];
  selectedId: string | null;
  order: Order;
  onSelect: (id: string) => void;
  onCreate: () => void;
  onOrderChange: (order: Order) => void;
};

export function NoteList({ notes, selectedId, order, onSelect, onCreate, onOrderChange }: Props) {
  return (
    <nav className="note-nav" aria-label="Notizen">
      <div className="sidebar-head">
        <button type="button" className="primary" onClick={onCreate}>
          Neue Notiz
        </button>
        <select
          aria-label="Sortierung"
          value={order}
          onChange={(e) => onOrderChange(e.target.value as Order)}
        >
          <option value="recent">Zuletzt geändert</option>
          <option value="title">Nach Titel</option>
        </select>
      </div>

      {notes.length === 0 ? (
        <p className="muted sidebar-empty">Noch keine Notizen.</p>
      ) : (
        <ul className="note-list">
          {notes.map((note) => (
            <li key={note.id}>
              <button
                type="button"
                className="note-item"
                aria-current={note.id === selectedId ? "true" : undefined}
                onClick={() => onSelect(note.id)}
              >
                <span className="note-title">{note.title}</span>
                <span className="note-excerpt">{excerpt(note.body) || "—"}</span>
                <span className="note-date">{formatDateTime(note.updated_at)}</span>
              </button>
            </li>
          ))}
        </ul>
      )}
    </nav>
  );
}

// Was rechts steht, solange keine Notiz ausgewählt ist.

type Props = {
  hasNotes: boolean;
  onCreate: () => void;
  onCreateExample: () => void;
};

export function EmptyState({ hasNotes, onCreate, onCreateExample }: Props) {
  return (
    <section className="empty">
      <h1>{hasNotes ? "Keine Notiz ausgewählt" : "Willkommen"}</h1>
      <p className="muted">
        Notizen in Markdown, mit Diagrammen aus <code>```mermaid</code>-Blöcken. Dieselben Daten
        sieht die Kommandozeile: <code>just cli note list</code>
      </p>
      <div className="actions">
        <button type="button" className="primary" onClick={onCreate}>
          Neue Notiz
        </button>
        <button type="button" className="ghost" onClick={onCreateExample}>
          Beispiel mit Diagramm anlegen
        </button>
      </div>
    </section>
  );
}

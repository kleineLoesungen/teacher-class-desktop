// Was rechts steht, solange keine Klasse ausgewählt ist.

type Props = {
  hasClasses: boolean;
  onCreate: () => void;
};

export function EmptyState({ hasClasses, onCreate }: Props) {
  return (
    <section className="empty">
      <h1>{hasClasses ? "Keine Klasse ausgewählt" : "Willkommen"}</h1>
      <p className="muted">
        Lege Klassen mit ihren Schülern an. Darauf bauen später Prüfungen und der Excel-Export auf.
        Dieselben Daten sieht die Kommandozeile: <code>just cli class list</code>
      </p>
      <div className="actions">
        <button type="button" className="primary" onClick={onCreate}>
          Neue Klasse
        </button>
      </div>
    </section>
  );
}

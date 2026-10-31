// Was rechts steht, solange nichts ausgewählt ist.

type Props = {
  title: string;
  text: string;
  /** Beschriftung des Knopfes, z. B. „Neue Klasse“. */
  action: string;
  onCreate: () => void;
};

export function EmptyState({ title, text, action, onCreate }: Props) {
  return (
    <section className="empty">
      <h1>{title}</h1>
      <p className="muted">{text}</p>
      <div className="actions">
        <button type="button" className="primary" onClick={onCreate}>
          {action}
        </button>
      </div>
    </section>
  );
}

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
      <svg
        className="empty-icon"
        viewBox="0 0 24 24"
        fill="none"
        stroke="currentColor"
        strokeWidth="1.8"
        strokeLinecap="round"
        strokeLinejoin="round"
        aria-hidden="true"
      >
        <path d="M4 5h16v14H4z" />
        <path d="M8 10h8M8 14h5" />
      </svg>
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

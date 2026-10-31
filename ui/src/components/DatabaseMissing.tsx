// Statt der Arbeitsfläche, wenn am gewählten Ort keine Datenbank liegt.
//
// Die Oberfläche legt hier nichts still neu an: Der Benutzer wählt, ob er eine
// vorhandene Datenbank nimmt (der Stick war nur nicht eingesteckt), eine neue
// anlegt oder zum Standardort zurückkehrt. Was erlaubt ist, entscheidet Rust —
// die Meldung zum Feld `db_file` kommt von dort.

import { FieldMessage } from "./FieldMessage";

type Props = {
  /** Die Meldung aus Rust, nennt den Ort. */
  message: string;
  fieldMessage: (field: string) => string | undefined;
  onUse: () => void;
  onCreate: () => void;
  onReset: () => void;
};

export function DatabaseMissing({ message, fieldMessage, onUse, onCreate, onReset }: Props) {
  return (
    <section className="empty">
      <h1>Datenbank nicht gefunden</h1>
      <p className="muted">{message}</p>
      <p className="muted">
        Ist der Datenträger eingesteckt? Sonst wähle eine vorhandene Datenbank, lege eine neue an
        oder kehre zum Standardort zurück.
      </p>
      <FieldMessage id="db_file-error" message={fieldMessage("db_file")} />
      <div className="actions">
        <button type="button" onClick={onReset}>
          Standardort verwenden
        </button>
        <button type="button" onClick={onCreate}>
          Neue Datenbank anlegen …
        </button>
        <button type="button" className="primary" onClick={onUse}>
          Vorhandene Datenbank wählen …
        </button>
      </div>
    </section>
  );
}

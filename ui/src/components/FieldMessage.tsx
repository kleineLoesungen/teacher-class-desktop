// Eine Meldung direkt unter einem Eingabefeld.
//
// Eingabefehler gehören ans Feld, nicht oben ins Fenster — sonst sucht der
// Benutzer, welches Feld gemeint ist. Die Meldung selbst kommt aus Rust.

type Props = { id: string; message: string | undefined };

export function FieldMessage({ id, message }: Props) {
  if (!message) return null;
  return (
    <p id={id} className="field-message" role="alert">
      {message}
    </p>
  );
}

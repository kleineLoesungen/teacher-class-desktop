// Rückfrage vor etwas, das sich nicht rückgängig machen lässt.

import { Dialog } from "./Dialog";

type Props = {
  title: string;
  message: string;
  /** Nicht noch einmal „Löschen" — zwei gleichnamige Knöpfe sind für
   *  Bildschirmleser und Tests mehrdeutig. */
  confirmLabel: string;
  onConfirm: () => void;
  onCancel: () => void;
};

export function ConfirmDialog({ title, message, confirmLabel, onConfirm, onCancel }: Props) {
  return (
    <Dialog title={title} onClose={onCancel}>
      <p>{message}</p>
      <div className="actions">
        <button type="button" className="ghost" onClick={onCancel}>
          Abbrechen
        </button>
        <button type="button" className="danger" onClick={onConfirm}>
          {confirmLabel}
        </button>
      </div>
    </Dialog>
  );
}

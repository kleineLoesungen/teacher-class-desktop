// Bearbeiten und Vorschau einer Notiz.
//
// Zeichnet nur. Der Entwurf gehört App.tsx; hier wird er angezeigt und jede
// Änderung als neuer Entwurf nach oben gemeldet.

import type { NoteInput } from "../bindings";
import { FieldMessage } from "./FieldMessage";
import { MarkdownView } from "./MarkdownView";

export type EditorMode = "edit" | "split" | "preview";

type Props = {
  draft: NoteInput;
  isNew: boolean;
  dirty: boolean;
  saving: boolean;
  mode: EditorMode;
  dark: boolean;
  fieldMessage: (field: string) => string | undefined;
  onChange: (draft: NoteInput) => void;
  onModeChange: (mode: EditorMode) => void;
  onSave: () => void;
  onDelete: () => void;
  onOpenLink: (url: string) => void;
  previewDelayMs?: number;
};

const MODES: [EditorMode, string][] = [
  ["edit", "Bearbeiten"],
  ["split", "Geteilt"],
  ["preview", "Vorschau"],
];

export function NoteEditor(props: Props) {
  const { draft, isNew, dirty, saving, mode, dark, fieldMessage } = props;
  const titleError = fieldMessage("title");
  const bodyError = fieldMessage("body");

  return (
    <section className="editor" aria-label="Notiz">
      <header className="editor-head">
        <div className="field grow">
          <input
            className="title-input"
            aria-label="Titel"
            placeholder="Titel"
            value={draft.title}
            aria-invalid={titleError ? true : undefined}
            aria-describedby={titleError ? "title-error" : undefined}
            onChange={(e) => props.onChange({ ...draft, title: e.target.value })}
          />
          <FieldMessage id="title-error" message={titleError} />
        </div>

        <fieldset className="segmented">
          <legend className="visually-hidden">Ansicht</legend>
          {MODES.map(([value, label]) => (
            <button
              key={value}
              type="button"
              aria-pressed={mode === value}
              onClick={() => props.onModeChange(value)}
            >
              {label}
            </button>
          ))}
        </fieldset>

        {!isNew && (
          <button type="button" className="ghost" onClick={props.onDelete}>
            Löschen
          </button>
        )}
        <button
          type="button"
          className="primary"
          disabled={saving || (!dirty && !isNew)}
          onClick={props.onSave}
        >
          {saving ? "Speichert …" : "Speichern"}
        </button>
      </header>

      <div className={`editor-body mode-${mode}`}>
        {mode !== "preview" && (
          <div className="field pane">
            <textarea
              className="body-input"
              aria-label="Text"
              placeholder={"Markdown. Diagramme als ```mermaid-Block."}
              value={draft.body}
              spellCheck
              aria-invalid={bodyError ? true : undefined}
              aria-describedby={bodyError ? "body-error" : undefined}
              onChange={(e) => props.onChange({ ...draft, body: e.target.value })}
            />
            <FieldMessage id="body-error" message={bodyError} />
          </div>
        )}
        {mode !== "edit" && (
          <div className="pane preview">
            <MarkdownView
              source={draft.body}
              dark={dark}
              onOpenLink={props.onOpenLink}
              {...(props.previewDelayMs !== undefined && { delayMs: props.previewDelayMs })}
            />
          </div>
        )}
      </div>
    </section>
  );
}

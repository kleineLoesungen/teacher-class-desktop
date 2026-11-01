// Der Druck aus der Bewertung: Klasse wählen, Vorlage wählen, drucken.
//
// Zeichnet nur. Bewusst ein Dialog mit Radioknöpfen statt eines Aufklappmenüs:
// Die Auswahl bleibt sichtbar stehen, gedruckt wird erst auf den einen Knopf, und
// es gibt keine Klick-und-Fokus-Logik, die in WebKit anders läuft als in Chromium.

import { useState } from "react";
import type { Class, TemplateInfo } from "../bindings";
import { classLabel } from "../lib/format";
import { Dialog } from "./Dialog";

type Props = {
  /** Die Klassen der Prüfung. */
  classes: Class[];
  /** Die Klasse, die in der Bewertung gerade offen ist. */
  classId: string | null;
  templates: TemplateInfo[];
  /** Der Ordner, in dem man eigene Vorlagen ablegt. */
  templateDir: string;
  /** Dateiname der gewählten Vorlage. */
  template: string;
  onSelectTemplate: (file: string) => void;
  onPrint: (classId: string) => void;
  onClose: () => void;
};

/** Der Name einer Vorlage, wie er in der Auswahl steht. */
function templateLabel(t: TemplateInfo): string {
  return t.standard ? "Standard" : t.name;
}

/** Eine Gruppe von Vorlagen; fehlt die Sorte, entfällt die Gruppe. */
function TemplateGroup({
  label,
  templates,
  selected,
  onSelect,
}: {
  label: string;
  templates: TemplateInfo[];
  selected: string;
  onSelect: (file: string) => void;
}) {
  if (templates.length === 0) return null;
  return (
    <fieldset className="check-list print-choice">
      <legend>{label}</legend>
      {templates.map((t) => (
        <label key={t.file} className="check">
          <input
            type="radio"
            name="print-template"
            checked={t.file === selected}
            onChange={() => onSelect(t.file)}
          />
          {templateLabel(t)}
        </label>
      ))}
    </fieldset>
  );
}

export function PrintDialog({
  classes,
  classId,
  templates,
  templateDir,
  template,
  onSelectTemplate,
  onPrint,
  onClose,
}: Props) {
  const [target, setTarget] = useState(classId ?? classes[0]?.id ?? "");

  return (
    <Dialog title="Als PDF drucken" onClose={onClose}>
      <form
        className="form"
        onSubmit={(e) => {
          e.preventDefault();
          onClose();
          onPrint(target);
        }}
      >
        <fieldset className="check-list print-choice">
          <legend>Klasse</legend>
          {classes.map((c) => (
            <label key={c.id} className="check">
              <input
                type="radio"
                name="print-class"
                checked={c.id === target}
                onChange={() => setTarget(c.id)}
              />
              {classLabel(c)}
            </label>
          ))}
        </fieldset>

        <TemplateGroup
          label="Vorlage: eine Seite je Schüler"
          templates={templates.filter((t) => t.kind === "student")}
          selected={template}
          onSelect={onSelectTemplate}
        />
        <TemplateGroup
          label="Vorlage: eine Übersicht je Klasse"
          templates={templates.filter((t) => t.kind === "class")}
          selected={template}
          onSelect={onSelectTemplate}
        />

        {templateDir !== "" && (
          <p className="muted print-dir">
            Eigene Vorlagen (Markdown-Dateien) liegen in {templateDir}. Welche Platzhalter es gibt,
            steht dort in placeholder.md.
          </p>
        )}

        <div className="actions">
          <button type="button" className="ghost" onClick={onClose}>
            Abbrechen
          </button>
          <button type="submit" className="primary">
            PDF speichern …
          </button>
        </div>
      </form>
    </Dialog>
  );
}

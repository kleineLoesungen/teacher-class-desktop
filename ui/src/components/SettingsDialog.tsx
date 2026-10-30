// Einstellungen. Dieselbe Datei, die `starter config set` schreibt.
//
// Geprüft wird in Rust (Config::validate). Diese Ansicht zeigt nur an, was
// von dort als Feldfehler zurückkommt.

import { useState } from "react";
import type { AppInfo, Config, Theme } from "../bindings";
import { Dialog } from "./Dialog";
import { FieldMessage } from "./FieldMessage";

type Props = {
  config: Config;
  info: AppInfo | null;
  fieldMessage: (field: string) => string | undefined;
  onSave: (config: Config) => void;
  onClose: () => void;
};

const THEMES: [Theme, string][] = [
  ["system", "Systemeinstellung"],
  ["light", "Hell"],
  ["dark", "Dunkel"],
];

export function SettingsDialog({ config, info, fieldMessage, onSave, onClose }: Props) {
  const [draft, setDraft] = useState<Config>(config);

  return (
    <Dialog title="Einstellungen" onClose={onClose}>
      <form
        className="form"
        onSubmit={(e) => {
          e.preventDefault();
          onSave(draft);
        }}
      >
        <label>
          Name der Anwendung
          <input
            value={draft.app_name}
            onChange={(e) => setDraft({ ...draft, app_name: e.target.value })}
            aria-describedby="app_name-error"
          />
        </label>
        <FieldMessage id="app_name-error" message={fieldMessage("app_name")} />

        <label>
          Aussehen
          <select
            value={draft.theme}
            onChange={(e) => setDraft({ ...draft, theme: e.target.value as Theme })}
          >
            {THEMES.map(([value, label]) => (
              <option key={value} value={value}>
                {label}
              </option>
            ))}
          </select>
        </label>

        <label>
          Akzentfarbe
          <span className="row">
            <input
              type="color"
              aria-label="Akzentfarbe wählen"
              // Die Farbauswahl kennt nur #rrggbb. Solange im Textfeld etwas
              // Halbfertiges steht, zeigt sie die zuletzt gespeicherte Farbe.
              value={/^#[0-9a-f]{6}$/i.test(draft.accent) ? draft.accent : config.accent}
              onChange={(e) => setDraft({ ...draft, accent: e.target.value })}
            />
            <input
              className="grow"
              aria-label="Akzentfarbe"
              value={draft.accent}
              onChange={(e) => setDraft({ ...draft, accent: e.target.value })}
              aria-describedby="accent-error"
            />
          </span>
        </label>
        <FieldMessage id="accent-error" message={fieldMessage("accent")} />

        {info && (
          <dl className="info">
            <dt>Fassung</dt>
            <dd>{info.version}</dd>
            <dt>Datenbank</dt>
            <dd>{info.db_file}</dd>
            <dt>Einstellungen</dt>
            <dd>{info.config_file}</dd>
            <dt>Protokoll</dt>
            <dd>{info.log_file}</dd>
          </dl>
        )}

        <div className="actions">
          <button type="button" className="ghost" onClick={onClose}>
            Abbrechen
          </button>
          <button type="submit" className="primary">
            Übernehmen
          </button>
        </div>
      </form>
    </Dialog>
  );
}

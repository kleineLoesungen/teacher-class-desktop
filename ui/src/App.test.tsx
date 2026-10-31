// Die Oberfläche ohne Fenster durchklicken.
//
// `mockIPC` ersetzt die Rust-Seite: Jeder `invoke` aus api.ts landet in
// `backend` unten. So wird geprüft, dass die Oberfläche die richtigen Befehle
// mit den richtigen Argumentnamen schickt und Antworten richtig anzeigt.
//
// Die Attrappe hat absichtlich KEINE eigenen Regeln — sie lehnt nur ab, was
// der Test ihr sagt. Die Regeln stehen in crates/core und werden dort
// geprüft. Ob die echte Rust-Seite dieselben Namen erwartet, prüft
// crates/desktop/tests/ipc_contract.rs.

import { mockIPC, mockWindows } from "@tauri-apps/api/mocks";
import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it } from "vitest";
import { App, EXAMPLE_NOTE } from "./App";
import type { ApiError, Config, Note, NoteInput } from "./bindings";

type Call = { cmd: string; args: Record<string, unknown> };

class Rejection {
  constructor(readonly error: ApiError) {}
}

function note(id: string, input: NoteInput): Note {
  const stamp = "2026-09-15T06:39:51Z";
  return { id, ...input, created_at: stamp, updated_at: stamp };
}

let notes: Note[];
let calls: Call[];
let config: Config;
let nextError: ApiError | null;

function backend(cmd: string, args: Record<string, unknown> = {}): unknown {
  calls.push({ cmd, args });
  if (nextError) {
    const error = nextError;
    nextError = null;
    throw new Rejection(error);
  }
  switch (cmd) {
    case "config_get":
      return config;
    case "config_set":
      config = args.config as Config;
      return config;
    case "app_info":
      return {
        version: "0.1.0",
        data_dir: "/tmp/d",
        config_file: "/tmp/c",
        db_file: "/tmp/db",
        log_file: "/tmp/l",
      };
    case "note_list":
      return notes;
    case "note_get":
      return notes.find((n) => n.id === args.id);
    case "note_create": {
      const created = note(`id-${notes.length + 1}`, args.input as NoteInput);
      notes = [created, ...notes];
      return created;
    }
    case "note_update": {
      const updated = note(args.id as string, args.input as NoteInput);
      notes = notes.map((n) => (n.id === updated.id ? updated : n));
      return updated;
    }
    case "note_delete":
      notes = notes.filter((n) => n.id !== args.id);
      return null;
    default:
      // Fenster-API und Plugins: annehmen, nichts tun.
      return null;
  }
}

beforeEach(() => {
  notes = [];
  calls = [];
  nextError = null;
  config = { app_name: "Teacher Class", theme: "light", accent: "#3b6ea5" };
  mockWindows("main");
  mockIPC((cmd, args) => {
    try {
      return backend(cmd, args as Record<string, unknown>);
    } catch (e) {
      // Ein abgelehntes Promise mit der Fehlerhülle — wie aus Rust.
      if (e instanceof Rejection) return Promise.reject(e.error);
      throw e;
    }
  });
});

function commands(name: string): Call[] {
  return calls.filter((c) => c.cmd === name);
}

describe("Oberfläche", () => {
  it("lädt beim Start Einstellungen und Notizen", async () => {
    notes = [note("a", { title: "Einkauf", body: "Milch" })];
    render(<App previewDelayMs={0} />);

    expect(await screen.findByRole("button", { name: /Einkauf/ })).toBeTruthy();
    expect(commands("note_list")[0]?.args).toEqual({ order: "recent" });
    // waitFor, nicht expect: Das Aussehen setzt ein useEffect, nachdem die
    // Einstellungen angekommen sind. Dass die Notizen schon dastehen, heißt
    // nicht, dass React diesen Effekt bereits ausgeführt hat — auf einem
    // langsamen Rechner liegt dazwischen ein Zeichendurchgang. Genau daran
    // ist dieser Test in der CI auf Windows hängen geblieben.
    await waitFor(() => expect(document.documentElement.dataset.theme).toBe("light"));
  });

  it("legt eine Notiz an und schickt den Entwurf als `input`", async () => {
    const user = userEvent.setup();
    render(<App previewDelayMs={0} />);

    await user.click(
      (await screen.findAllByRole("button", { name: "Neue Notiz" }))[0] as HTMLElement,
    );
    await user.type(screen.getByRole("textbox", { name: "Titel" }), "Einkauf");
    await user.type(screen.getByRole("textbox", { name: "Text" }), "Milch");
    await user.click(screen.getByRole("button", { name: "Speichern" }));

    await waitFor(() => expect(commands("note_create")).toHaveLength(1));
    expect(commands("note_create")[0]?.args).toEqual({
      input: { title: "Einkauf", body: "Milch" },
    });
    expect(await screen.findByRole("button", { name: /Einkauf/ })).toBeTruthy();
  });

  it("zeigt einen Eingabefehler am Feld, nicht oben", async () => {
    const user = userEvent.setup();
    render(<App previewDelayMs={0} />);

    await user.click(
      (await screen.findAllByRole("button", { name: "Neue Notiz" }))[0] as HTMLElement,
    );
    nextError = {
      code: "validation_failed",
      message: "title: darf nicht leer sein",
      fields: [{ field: "title", message: "darf nicht leer sein" }],
    };
    await user.click(screen.getByRole("button", { name: "Speichern" }));

    const title = screen.getByRole("textbox", { name: "Titel" });
    await waitFor(() => expect(title.getAttribute("aria-invalid")).toBe("true"));
    expect(screen.getByText("darf nicht leer sein")).toBeTruthy();
    expect(document.querySelector(".banner")).toBeNull();
  });

  it("zeigt einen Programmfehler oben", async () => {
    nextError = { code: "internal_error", message: "Datenbankfehler: disk full" };
    render(<App previewDelayMs={0} />);
    expect(await screen.findByText("Datenbankfehler: disk full")).toBeTruthy();
  });

  it("löscht erst nach der Rückfrage", async () => {
    const user = userEvent.setup();
    notes = [note("a", { title: "Weg damit", body: "" })];
    render(<App previewDelayMs={0} />);

    await user.click(await screen.findByRole("button", { name: /Weg damit/ }));
    await user.click(screen.getByRole("button", { name: "Löschen" }));
    expect(commands("note_delete")).toHaveLength(0);

    await user.click(screen.getByRole("button", { name: "Endgültig löschen" }));
    await waitFor(() => expect(commands("note_delete")[0]?.args).toEqual({ id: "a" }));
  });

  it("legt die Beispielnotiz mit Diagramm an", async () => {
    const user = userEvent.setup();
    render(<App previewDelayMs={0} />);

    await user.click(await screen.findByRole("button", { name: "Beispiel mit Diagramm anlegen" }));
    await waitFor(() => expect(commands("note_create")[0]?.args).toEqual({ input: EXAMPLE_NOTE }));
  });

  it("öffnet Verweise über das Opener-Plugin statt im Fenster", async () => {
    const user = userEvent.setup();
    notes = [note("a", { title: "Mit Verweis", body: "[Doku](https://tauri.app)" })];
    render(<App previewDelayMs={0} />);

    await user.click(await screen.findByRole("button", { name: /Mit Verweis/ }));
    const link = await screen.findByRole("link", { name: "Doku" });
    await user.click(link);

    await waitFor(() => expect(commands("plugin:opener|open_url")).toHaveLength(1));
    expect(commands("plugin:opener|open_url")[0]?.args).toMatchObject({ url: "https://tauri.app" });
  });

  it("speichert Einstellungen und wendet die Akzentfarbe an", async () => {
    const user = userEvent.setup();
    render(<App previewDelayMs={0} />);

    await user.click(await screen.findByRole("button", { name: "Einstellungen" }));
    const accent = screen.getByRole("textbox", { name: "Akzentfarbe" });
    await user.clear(accent);
    await user.type(accent, "#ffdd00");
    await user.click(screen.getByRole("button", { name: "Übernehmen" }));

    await waitFor(() => expect(commands("config_set")).toHaveLength(1));
    // Auch hier: Der Befehl ist durch, der Effekt noch nicht zwingend.
    await waitFor(() => {
      expect(document.documentElement.style.getPropertyValue("--accent")).toBe("#ffdd00");
      expect(document.documentElement.dataset.accentLight).toBe("true");
    });
  });
});

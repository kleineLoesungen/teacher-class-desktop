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
import { App } from "./App";
import type { ApiError, Class, ClassInput, Config, Student } from "./bindings";

type Call = { cmd: string; args: Record<string, unknown> };

class Rejection {
  constructor(readonly error: ApiError) {}
}

function cls(id: string, input: ClassInput, studentCount = 0): Class {
  return { id, ...input, student_count: studentCount, created_at: "2026-09-15T06:39:51Z" };
}

function student(id: string, classId: string, last: string, first: string): Student {
  return { id, class_id: classId, last_name: last, first_name: first };
}

let classes: Class[];
let students: Student[];
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
    case "class_list":
      return classes.map((c) => ({
        ...c,
        student_count: students.filter((s) => s.class_id === c.id).length,
      }));
    case "class_create": {
      const created = cls(`id-${classes.length + 1}`, args.input as ClassInput);
      classes = [created, ...classes];
      return created;
    }
    case "class_update": {
      const updated = cls(args.id as string, args.input as ClassInput);
      classes = classes.map((c) => (c.id === updated.id ? updated : c));
      return updated;
    }
    case "class_delete":
      classes = classes.filter((c) => c.id !== args.id);
      return null;
    case "student_list":
      return students.filter((s) => s.class_id === args.id);
    case "student_add": {
      const added = (args.text as string)
        .split("\n")
        .filter((line) => line.trim() !== "")
        .map((line, i) => {
          const [last = "", first = ""] = line.split(",").map((p) => p.trim());
          return student(`s-${students.length + i + 1}`, args.id as string, last, first);
        });
      students = [...students, ...added];
      return added;
    }
    case "student_delete":
      students = students.filter((s) => s.id !== args.id);
      return null;
    default:
      // Fenster-API und Plugins: annehmen, nichts tun.
      return null;
  }
}

beforeEach(() => {
  classes = [];
  students = [];
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
  it("lädt beim Start Einstellungen und Klassen", async () => {
    classes = [cls("a", { name: "7a", year: 2025 })];
    render(<App />);

    expect(await screen.findByRole("button", { name: /7a \(2025\)/ })).toBeTruthy();
    // waitFor, nicht expect: Das Aussehen setzt ein useEffect, nachdem die
    // Einstellungen angekommen sind. Dass die Klassen schon dastehen, heißt
    // nicht, dass React diesen Effekt bereits ausgeführt hat — auf einem
    // langsamen Rechner liegt dazwischen ein Zeichendurchgang. Genau daran
    // ist dieser Test in der CI auf Windows hängen geblieben.
    await waitFor(() => expect(document.documentElement.dataset.theme).toBe("light"));
  });

  it("legt eine Klasse an und schickt den Entwurf als `input`", async () => {
    const user = userEvent.setup();
    render(<App />);

    await user.click(
      (await screen.findAllByRole("button", { name: "Neue Klasse" }))[0] as HTMLElement,
    );
    await user.type(screen.getByRole("textbox", { name: "Klassenname" }), "7a");
    const year = screen.getByRole("textbox", { name: "Jahr" });
    await user.clear(year);
    await user.type(year, "2025");
    await user.click(screen.getByRole("button", { name: "Speichern" }));

    await waitFor(() => expect(commands("class_create")).toHaveLength(1));
    expect(commands("class_create")[0]?.args).toEqual({ input: { name: "7a", year: 2025 } });
    expect(await screen.findByRole("button", { name: /7a \(2025\)/ })).toBeTruthy();
  });

  it("zeigt einen Eingabefehler am Feld, nicht oben", async () => {
    const user = userEvent.setup();
    render(<App />);

    await user.click(
      (await screen.findAllByRole("button", { name: "Neue Klasse" }))[0] as HTMLElement,
    );
    nextError = {
      code: "validation_failed",
      message: "name: darf nicht leer sein",
      fields: [{ field: "name", message: "darf nicht leer sein" }],
    };
    await user.click(screen.getByRole("button", { name: "Speichern" }));

    const name = screen.getByRole("textbox", { name: "Klassenname" });
    await waitFor(() => expect(name.getAttribute("aria-invalid")).toBe("true"));
    expect(screen.getByText("darf nicht leer sein")).toBeTruthy();
    expect(document.querySelector(".banner")).toBeNull();
  });

  it("zeigt einen Programmfehler oben", async () => {
    nextError = { code: "internal_error", message: "Datenbankfehler: disk full" };
    render(<App />);
    expect(await screen.findByText("Datenbankfehler: disk full")).toBeTruthy();
  });

  it("zeigt die Schüler einer Klasse", async () => {
    const user = userEvent.setup();
    classes = [cls("a", { name: "7a", year: 2025 })];
    students = [student("s1", "a", "Muster", "Max")];
    render(<App />);

    await user.click(await screen.findByRole("button", { name: /7a \(2025\)/ }));

    expect(await screen.findByText("Muster, Max")).toBeTruthy();
    expect(commands("student_list")[0]?.args).toEqual({ id: "a" });
  });

  it("schickt mehrere Schüler als ein Text mit `id` und `text`", async () => {
    const user = userEvent.setup();
    classes = [cls("a", { name: "7a", year: 2025 })];
    render(<App />);

    await user.click(await screen.findByRole("button", { name: /7a \(2025\)/ }));
    await user.type(
      await screen.findByRole("textbox", { name: "Schüler hinzufügen" }),
      "Muster, Max{Enter}Beispiel, Eva",
    );
    await user.click(screen.getByRole("button", { name: "Hinzufügen" }));

    await waitFor(() => expect(commands("student_add")).toHaveLength(1));
    expect(commands("student_add")[0]?.args).toEqual({
      id: "a",
      text: "Muster, Max\nBeispiel, Eva",
    });
    expect(await screen.findByText("Beispiel, Eva")).toBeTruthy();
  });

  it("zeigt den Fehler zu den Schülerzeilen am Eingabefeld", async () => {
    const user = userEvent.setup();
    classes = [cls("a", { name: "7a", year: 2025 })];
    render(<App />);

    await user.click(await screen.findByRole("button", { name: /7a \(2025\)/ }));
    await user.type(
      await screen.findByRole("textbox", { name: "Schüler hinzufügen" }),
      "ohne Komma",
    );
    nextError = {
      code: "validation_failed",
      message: "students: Zeile 1",
      fields: [{ field: "students", message: "Zeile 1: erwartet „Nachname, Vorname“" }],
    };
    await user.click(screen.getByRole("button", { name: "Hinzufügen" }));

    expect(await screen.findByText(/Zeile 1: erwartet/)).toBeTruthy();
    expect(document.querySelector(".banner")).toBeNull();
  });

  it("löscht eine Klasse erst nach der Rückfrage", async () => {
    const user = userEvent.setup();
    classes = [cls("a", { name: "7a", year: 2025 })];
    render(<App />);

    await user.click(await screen.findByRole("button", { name: /7a \(2025\)/ }));
    await user.click(screen.getByRole("button", { name: "Klasse löschen" }));
    expect(commands("class_delete")).toHaveLength(0);

    await user.click(screen.getByRole("button", { name: "Endgültig löschen" }));
    await waitFor(() => expect(commands("class_delete")[0]?.args).toEqual({ id: "a" }));
  });

  it("entfernt einen Schüler erst nach der Rückfrage", async () => {
    const user = userEvent.setup();
    classes = [cls("a", { name: "7a", year: 2025 })];
    students = [student("s1", "a", "Muster", "Max")];
    render(<App />);

    await user.click(await screen.findByRole("button", { name: /7a \(2025\)/ }));
    await user.click(await screen.findByRole("button", { name: "Muster, Max entfernen" }));
    expect(commands("student_delete")).toHaveLength(0);

    await user.click(screen.getByRole("button", { name: "Endgültig entfernen" }));
    await waitFor(() => expect(commands("student_delete")[0]?.args).toEqual({ id: "s1" }));
  });

  it("speichert Einstellungen und wendet die Akzentfarbe an", async () => {
    const user = userEvent.setup();
    render(<App />);

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

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
import type {
  ApiError,
  Class,
  ClassInput,
  Config,
  Exam,
  ExamInput,
  ScoreInput,
  Student,
  StudentResult,
} from "./bindings";

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
let exams: Exam[];
let results: StudentResult[];
/** Was der Speichern-Dialog liefert; `null` = abgebrochen. */
let savePath: string | null;
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
    case "exam_list":
      return exams;
    case "exam_create":
    case "exam_update": {
      // Die Attrappe rechnet nichts nach: „bis“ und die Summen hat Rust
      // errechnet — hier stehen feste Werte, damit die Anzeige geprüft wird.
      const input = args.input as ExamInput;
      const exam: Exam = {
        id: cmd === "exam_create" ? `e-${exams.length + 1}` : (args.id as string),
        name: input.name,
        class_ids: input.class_ids,
        groups: input.groups.map((g) => ({
          name: g.name,
          items: g.items.map((i) => ({ ...i, description: i.description ?? "" })),
          max_points: g.items.reduce((sum, i) => sum + i.max_points, 0),
        })),
        grades: [{ name: "6", from_points: 0, to_points: 9.5 }],
        max_points: 20,
        with_notes: input.with_notes ?? false,
        created_at: "2026-09-15T06:39:51Z",
      };
      exams =
        cmd === "exam_create" ? [exam, ...exams] : exams.map((e) => (e.id === exam.id ? exam : e));
      return exam;
    }
    case "exam_delete":
      exams = exams.filter((e) => e.id !== args.id);
      return null;
    case "exam_results":
      return results;
    case "exam_score": {
      // Keine Regeln, keine Rechnung: Rust hat Summe und Note errechnet — hier
      // stehen feste Werte, damit die Anzeige geprüft wird.
      const input = args.input as ScoreInput;
      const updated: StudentResult = {
        ...(results.find((r) => r.student.id === input.student_id) as StudentResult),
        total: input.points ?? 0,
        grade: input.points === null ? null : "1",
      };
      results = results.map((r) => (r.student.id === updated.student.id ? updated : r));
      return updated;
    }
    case "plugin:dialog|save":
      return savePath;
    default:
      // Fenster-API und Plugins: annehmen, nichts tun.
      return null;
  }
}

beforeEach(() => {
  classes = [];
  students = [];
  exams = [];
  results = [];
  savePath = "/tmp/Klassenarbeit.xlsx";
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

  describe("Prüfungen", () => {
    async function openExams(user: ReturnType<typeof userEvent.setup>) {
      await user.click(await screen.findByRole("button", { name: "Prüfungen" }));
    }

    function savedExam(): Exam {
      return {
        id: "e1",
        name: "Klassenarbeit",
        class_ids: ["a"],
        groups: [
          {
            name: "Aufgabe 1",
            items: [{ name: "a", description: "Rechenweg angeben", max_points: 2.5 }],
            max_points: 2.5,
          },
        ],
        grades: [
          { name: "6", from_points: 0, to_points: 1.5 },
          { name: "1", from_points: 2, to_points: 2.5 },
        ],
        max_points: 2.5,
        with_notes: false,
        created_at: "2026-09-15T06:39:51Z",
      };
    }

    function maxResult(): StudentResult {
      return {
        student: student("s1", "a", "Muster", "Max"),
        groups: [{ scores: [{ points: null, note: "" }], points: 0 }],
        total: 0,
        grade: null,
      };
    }

    it("legt eine Prüfung an und schickt Punkte als Zahlen", async () => {
      const user = userEvent.setup();
      classes = [cls("a", { name: "7a", year: 2025 })];
      render(<App />);

      await openExams(user);
      await user.click(
        (await screen.findAllByRole("button", { name: "Neue Prüfung" }))[0] as HTMLElement,
      );
      await user.type(screen.getByRole("textbox", { name: "Name der Prüfung" }), "Klassenarbeit");
      await user.click(screen.getByRole("checkbox", { name: "7a (2025)" }));
      await user.type(screen.getByRole("textbox", { name: "Gruppe 1 Name" }), "Aufgabe 1");
      await user.type(screen.getByRole("textbox", { name: "Gruppe 1 Punkt 1 Name" }), "a");
      // Deutsches Komma, wie man Halbpunkte tippt.
      await user.type(
        screen.getByRole("textbox", { name: "Gruppe 1 Punkt 1 Höchstpunkte" }),
        "2,5",
      );
      await user.type(
        screen.getByRole("textbox", { name: "Gruppe 1 Punkt 1 Beschreibung" }),
        "Rechenweg angeben",
      );
      await user.click(
        screen.getByRole("checkbox", { name: "Zu jedem Prüfkriterium eine Begründung pflegen" }),
      );
      await user.type(screen.getByRole("textbox", { name: "Note 1 ab" }), "2");
      await user.click(screen.getByRole("button", { name: "Speichern" }));

      await waitFor(() => expect(commands("exam_create")).toHaveLength(1));
      const input = commands("exam_create")[0]?.args.input as ExamInput;
      expect(input.class_ids).toEqual(["a"]);
      expect(input.with_notes).toBe(true);
      expect(input.groups).toEqual([
        {
          name: "Aufgabe 1",
          items: [{ name: "a", description: "Rechenweg angeben", max_points: 2.5 }],
        },
      ]);
      // Sechs vorbelegte Noten; nur die 6 beginnt bei 0, die 1 bei dem, was getippt wurde.
      expect(input.grades).toHaveLength(6);
      expect(input.grades[0]).toEqual({ name: "1", from_points: 2 });
      expect(input.grades[5]).toEqual({ name: "6", from_points: 0 });
    });

    it("zeigt Fehler zu Schema und Matrix, wie Rust sie meldet", async () => {
      const user = userEvent.setup();
      classes = [cls("a", { name: "7a", year: 2025 })];
      render(<App />);

      await openExams(user);
      await user.click(
        (await screen.findAllByRole("button", { name: "Neue Prüfung" }))[0] as HTMLElement,
      );
      nextError = {
        code: "validation_failed",
        message: "x",
        fields: [
          { field: "groups", message: "Gruppe 1: Name darf nicht leer sein" },
          { field: "grades", message: "die niedrigste Note muss bei 0 Punkten beginnen" },
        ],
      };
      await user.click(screen.getByRole("button", { name: "Speichern" }));

      expect(await screen.findByText("Gruppe 1: Name darf nicht leer sein")).toBeTruthy();
      expect(screen.getByText("die niedrigste Note muss bei 0 Punkten beginnen")).toBeTruthy();
    });

    it("exportiert nach dem Speichern an den gewählten Ort", async () => {
      const user = userEvent.setup();
      classes = [cls("a", { name: "7a", year: 2025 })];
      exams = [savedExam()];
      render(<App />);

      await openExams(user);
      await user.click(await screen.findByRole("button", { name: /Klassenarbeit/ }));
      await user.click(screen.getByRole("button", { name: "Als Excel exportieren" }));

      await waitFor(() => expect(commands("exam_export")).toHaveLength(1));
      expect(commands("exam_export")[0]?.args).toEqual({
        id: "e1",
        path: "/tmp/Klassenarbeit.xlsx",
      });
      expect(await screen.findByText("Gespeichert: /tmp/Klassenarbeit.xlsx")).toBeTruthy();
      // Ohne Änderungen wird vor dem Export nicht gespeichert.
      expect(commands("exam_update")).toHaveLength(0);
    });

    it("exportiert nichts, wenn der Dialog abgebrochen wird", async () => {
      const user = userEvent.setup();
      classes = [cls("a", { name: "7a", year: 2025 })];
      exams = [savedExam()];
      savePath = null;
      render(<App />);

      await openExams(user);
      await user.click(await screen.findByRole("button", { name: /Klassenarbeit/ }));
      await user.click(screen.getByRole("button", { name: "Als Excel exportieren" }));

      await waitFor(() => expect(commands("plugin:dialog|save")).toHaveLength(1));
      expect(commands("exam_export")).toHaveLength(0);
    });

    it("zeigt die berechneten Werte nur für den gespeicherten Stand", async () => {
      const user = userEvent.setup();
      classes = [cls("a", { name: "7a", year: 2025 })];
      exams = [savedExam()];
      render(<App />);

      await openExams(user);
      await user.click(await screen.findByRole("button", { name: /Klassenarbeit/ }));
      expect(screen.getByText("max. 2,5")).toBeTruthy();
      expect(screen.getByText("1,5")).toBeTruthy();

      await user.type(screen.getByRole("textbox", { name: "Gruppe 1 Punkt 1 Höchstpunkte" }), "0");
      // Der Entwurf weicht ab — Rust hat dafür noch nichts gerechnet.
      expect(screen.queryByText("max. 2,5")).toBeNull();
      expect(screen.queryByText("1,5")).toBeNull();
    });

    it("trägt Punkte beim Verlassen des Feldes ein und zeigt, was Rust berechnet", async () => {
      const user = userEvent.setup();
      classes = [cls("a", { name: "7a", year: 2025 })];
      exams = [savedExam()];
      results = [maxResult()];
      render(<App />);

      await openExams(user);
      await user.click(await screen.findByRole("button", { name: /Klassenarbeit/ }));
      await user.click(screen.getByRole("button", { name: "Bewerten" }));
      // Ohne Änderungen am Entwurf wird vorher nicht gespeichert.
      expect(commands("exam_update")).toHaveLength(0);

      await user.click(await screen.findByRole("button", { name: /Muster, Max/ }));
      expect(screen.getByText("Rechenweg angeben")).toBeTruthy();
      // Ohne Begründungen gibt es kein Begründungsfeld.
      expect(screen.queryByRole("textbox", { name: /Begründung/ })).toBeNull();

      await user.type(screen.getByRole("textbox", { name: "Gruppe 1 Punkt 1 Punkte" }), "2,5");
      expect(commands("exam_score")).toHaveLength(0);
      await user.tab();

      await waitFor(() => expect(commands("exam_score")).toHaveLength(1));
      expect(commands("exam_score")[0]?.args).toEqual({
        id: "e1",
        input: { student_id: "s1", group: 0, item: 0, points: 2.5, note: "" },
      });
      expect(await screen.findByText("Gesamt: 2,5 / 2,5")).toBeTruthy();
      expect(screen.getByText("Note: 1")).toBeTruthy();
    });

    it("schickt Begründungen mit, wenn die Prüfung sie vorsieht", async () => {
      const user = userEvent.setup();
      classes = [cls("a", { name: "7a", year: 2025 })];
      exams = [{ ...savedExam(), with_notes: true }];
      results = [maxResult()];
      render(<App />);

      await openExams(user);
      await user.click(await screen.findByRole("button", { name: /Klassenarbeit/ }));
      await user.click(screen.getByRole("button", { name: "Bewerten" }));
      await user.click(await screen.findByRole("button", { name: /Muster, Max/ }));

      await user.type(screen.getByRole("textbox", { name: "Gruppe 1 Punkt 1 Punkte" }), "2");
      await user.type(
        screen.getByRole("textbox", { name: "Gruppe 1 Punkt 1 Begründung" }),
        "Vorzeichenfehler",
      );
      await user.tab();

      // Der Wechsel von den Punkten zur Begründung speichert die Punkte, das
      // Verlassen der Begründung speichert beides.
      await waitFor(() => expect(commands("exam_score")).toHaveLength(2));
      expect(commands("exam_score")[0]?.args.input).toMatchObject({ points: 2, note: "" });
      expect(commands("exam_score")[1]?.args.input).toEqual({
        student_id: "s1",
        group: 0,
        item: 0,
        points: 2,
        note: "Vorzeichenfehler",
      });
    });

    it("zeigt eine abgelehnte Eingabe mit Rusts Meldung", async () => {
      const user = userEvent.setup();
      classes = [cls("a", { name: "7a", year: 2025 })];
      exams = [savedExam()];
      results = [maxResult()];
      render(<App />);

      await openExams(user);
      await user.click(await screen.findByRole("button", { name: /Klassenarbeit/ }));
      await user.click(screen.getByRole("button", { name: "Bewerten" }));
      await user.click(await screen.findByRole("button", { name: /Muster, Max/ }));

      await user.type(screen.getByRole("textbox", { name: "Gruppe 1 Punkt 1 Punkte" }), "9");
      nextError = {
        code: "validation_failed",
        message: "x",
        fields: [{ field: "points", message: "Punkte müssen zwischen 0 und 2.5 liegen" }],
      };
      await user.tab();

      expect(await screen.findByText("Punkte müssen zwischen 0 und 2.5 liegen")).toBeTruthy();
    });

    it("löscht eine Prüfung erst nach der Rückfrage", async () => {
      const user = userEvent.setup();
      classes = [cls("a", { name: "7a", year: 2025 })];
      exams = [savedExam()];
      render(<App />);

      await openExams(user);
      await user.click(await screen.findByRole("button", { name: /Klassenarbeit/ }));
      await user.click(screen.getByRole("button", { name: "Prüfung löschen" }));
      expect(commands("exam_delete")).toHaveLength(0);

      await user.click(screen.getByRole("button", { name: "Endgültig löschen" }));
      await waitFor(() => expect(commands("exam_delete")[0]?.args).toEqual({ id: "e1" }));
    });
  });
});

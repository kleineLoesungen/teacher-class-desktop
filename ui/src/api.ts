// Die einzige Stelle, an der die Oberfläche mit Rust spricht.
//
// Jeder Befehl steht hier genau einmal, mit Typen aus bindings.ts. Eine
// Komponente importiert diese Datei nicht — sie gibt einen Rückruf nach oben,
// und App.tsx ruft. crates/core/tests/layering.rs bewacht beides.
//
// NEUER BEFEHL? Drei Stellen:
//   1. crates/desktop/src/commands.rs     #[tauri::command] async fn …
//   2. crates/desktop/src/lib.rs          generate_handler![…]
//   3. hier                               call<…>("name", { … })
// crates/desktop/tests/ipc_contract.rs prüft, dass keine fehlt.
//
// Argumentnamen: Tauri macht aus einem Rust-Argument `class_id` in JavaScript
// `noteId`. Die Befehle hier haben deshalb nur einwortige Argumente.

import { invoke } from "@tauri-apps/api/core";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { save } from "@tauri-apps/plugin-dialog";
import { openUrl } from "@tauri-apps/plugin-opener";
import type {
  ApiError as ApiErrorData,
  AppInfo,
  Class,
  ClassInput,
  Config,
  ErrorCode,
  Exam,
  ExamInput,
  FieldError,
  ScoreInput,
  Student,
  StudentInput,
  StudentResult,
} from "./bindings";

/** Ein abgelehnter Befehl. Dieselbe Form wie `error` im JSON der CLI. */
export class ApiError extends Error {
  readonly code: ErrorCode;
  readonly fields: FieldError[];

  constructor(data: ApiErrorData) {
    super(data.message);
    this.name = "ApiError";
    this.code = data.code;
    this.fields = data.fields ?? [];
  }

  /** Eingabefehler gehören ans Feld, alles andere oben ins Fenster. */
  get isValidation(): boolean {
    return this.fields.length > 0;
  }

  /** Die Meldung zu einem Feld, falls es eine gibt. */
  fieldMessage(field: string): string | undefined {
    return this.fields.find((f) => f.field === field)?.message;
  }

  /** Alle Meldungen zu einem Feld — `groups` und `grades` haben oft mehrere. */
  fieldMessages(field: string): string[] {
    return this.fields.filter((f) => f.field === field).map((f) => f.message);
  }

  /**
   * Macht aus allem, was ein Promise ablehnen kann, einen ApiError.
   *
   * Rust schickt ein Objekt mit `code`. Tauri selbst schickt bei einem
   * unbekannten Befehl oder fehlenden Argument nur einen Text.
   */
  static from(raw: unknown): ApiError {
    if (raw instanceof ApiError) return raw;
    if (raw && typeof raw === "object" && "code" in raw && "message" in raw) {
      return new ApiError(raw as ApiErrorData);
    }
    return new ApiError({ code: "internal_error", message: String(raw) });
  }
}

async function call<T>(cmd: string, args?: Record<string, unknown>): Promise<T> {
  try {
    return await invoke<T>(cmd, args);
  } catch (raw) {
    throw ApiError.from(raw);
  }
}

export const api = {
  classes: {
    list: () => call<Class[]>("class_list"),
    create: (input: ClassInput) => call<Class>("class_create", { input }),
    update: (id: string, input: ClassInput) => call<Class>("class_update", { id, input }),
    remove: (id: string) => call<null>("class_delete", { id }),
  },

  students: {
    /** Die Schüler der Klasse `classId`. */
    list: (classId: string) => call<Student[]>("student_list", { id: classId }),
    /** Eine Zeile `Nachname, Vorname` je Schüler. Alle oder keine. */
    add: (classId: string, text: string) => call<Student[]>("student_add", { id: classId, text }),
    update: (id: string, input: StudentInput) => call<Student>("student_update", { id, input }),
    remove: (id: string) => call<null>("student_delete", { id }),
  },

  exams: {
    list: () => call<Exam[]>("exam_list"),
    create: (input: ExamInput) => call<Exam>("exam_create", { input }),
    update: (id: string, input: ExamInput) => call<Exam>("exam_update", { id, input }),
    remove: (id: string) => call<null>("exam_delete", { id }),
    /** Schreibt die Excel-Datei nach `path` (aus `pickExportPath`). */
    export: (id: string, path: string) => call<null>("exam_export", { id, path }),
    /** Die Bewertung der Schüler der Klasse `classId` in der Prüfung `id`. */
    results: (id: string, classId: string) =>
      call<StudentResult[]>("exam_results", { id, class: classId }),
    /** Trägt Punkte und Begründung ein; liefert die neu berechnete Bewertung. */
    score: (id: string, input: ScoreInput) => call<StudentResult>("exam_score", { id, input }),
  },

  /**
   * Öffnet den Speichern-Dialog des Systems. `null`, wenn abgebrochen.
   * Braucht `dialog:allow-save` in crates/desktop/capabilities/.
   */
  pickExportPath: async (fileName: string): Promise<string | null> => {
    try {
      return await save({
        title: "Excel-Datei speichern",
        defaultPath: fileName,
        filters: [{ name: "Excel-Arbeitsmappe", extensions: ["xlsx"] }],
      });
    } catch (raw) {
      throw ApiError.from(raw);
    }
  },

  config: {
    get: () => call<Config>("config_get"),
    set: (config: Config) => call<Config>("config_set", { config }),
  },

  appInfo: () => call<AppInfo>("app_info"),

  /** Braucht `core:window:allow-set-title` in crates/desktop/capabilities/. */
  setWindowTitle: async (title: string) => {
    await getCurrentWindow().setTitle(title);
  },

  /**
   * Öffnet eine Adresse im Standardbrowser — nie im Fenster der Anwendung.
   * Welche Schemata erlaubt sind, steht in crates/desktop/capabilities/.
   */
  openExternal: async (url: string) => {
    try {
      await openUrl(url);
    } catch (raw) {
      throw ApiError.from(raw);
    }
  },
};

export type Api = typeof api;

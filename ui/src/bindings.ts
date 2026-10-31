// ERZEUGT aus den Rust-Typen — nicht von Hand ändern.
// Neu schreiben mit:  just bindings
// Quelle: crates/desktop/tests/bindings.rs

export type ApiError = { code: ErrorCode, message: string, fields?: Array<FieldError>, matches?: Array<string>, };

/**
 * Wo die Dateien liegen — dieselben Pfade wie `teacher-class paths --json`.
 */
export type AppInfo = { version: string, data_dir: string, config_file: string, db_file: string, log_file: string, };

export type Class = { id: string, name: string, year: number, 
/**
 * Wie viele Schüler in der Klasse sind.
 */
student_count: number, created_at: string, };

/**
 * Was von außen hereinkommt — ungeprüft.
 */
export type ClassInput = { name: string, year: number, };

export type Config = { 
/**
 * Fenstertitel und Name in der Oberfläche.
 */
app_name: string, 
/**
 * Hell, dunkel oder der Systemeinstellung folgen.
 */
theme: Theme, 
/**
 * Akzentfarbe als `#rrggbb`. Siehe docs/DESIGN.md.
 */
accent: string, };

/**
 * Maschinenlesbare Fehlerart.
 *
 * Dieselben Namen stehen im JSON der Kommandozeile (`error.code`) und in der
 * Fehlerhülle der Desktop-Schale. Ein Skript und die Oberfläche können sich
 * darauf verlassen — ein Name wird deshalb nie umbenannt, nur ergänzt.
 */
export type ErrorCode = "not_found" | "validation_failed" | "ambiguous_id" | "config_invalid" | "internal_error";

/**
 * Eine Meldung, die zu genau einem Eingabefeld gehört.
 *
 * `field` ist englisch und entspricht dem Feldnamen im Modell (`title`,
 * `body`). `message` ist deutsch und für Menschen.
 */
export type FieldError = { field: string, message: string, };

export type Student = { id: string, class_id: string, last_name: string, first_name: string, };

/**
 * Ein Schüler, ungeprüft.
 */
export type StudentInput = { last_name: string, first_name: string, };

export type Theme = "system" | "light" | "dark";

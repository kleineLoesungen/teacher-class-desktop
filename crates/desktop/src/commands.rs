//! Die Befehle, die das Webview aufrufen kann.
//!
//! **Jeder Befehl ist dünn:** Argumente annehmen, eine Funktion in
//! `teacher_class_core` rufen, das Ergebnis zurückgeben. Prüfung und SQL stehen in
//! core — sonst gälten sie für die Kommandozeile nicht.
//! `crates/core/tests/layering.rs` sucht hier nach `execute(` und `Validator`.
//!
//! Drei Tauri-Eigenheiten, die hier zusammenkommen (ausführlich in CLAUDE.md):
//!
//! 1. **`async fn`, nicht `fn`.** Ein synchroner Befehl läuft auf dem
//!    Hauptfaden und friert das Fenster ein, solange er arbeitet.
//! 2. **Argumentnamen werden in JavaScript zu camelCase.** `note_id` hieße im
//!    Webview `noteId`. Hier sind deshalb alle Argumente einwortig.
//! 3. **Neuer Befehl = drei Stellen:** hier, `generate_handler!` in `lib.rs`,
//!    `ui/src/api.ts`. `tests/ipc_contract.rs` prüft, dass keine fehlt.

use serde::Serialize;
use tauri::State;
use teacher_class_core::class::{self, Class, ClassInput};
use teacher_class_core::exam::{self, Exam, ExamInput, Preview};
use teacher_class_core::export;
use teacher_class_core::grading::{self, ScoreInput, StudentResult};
use teacher_class_core::student::{self, Student, StudentInput};
use teacher_class_core::{Config, Paths};
use ts_rs::TS;
use uuid::Uuid;

use crate::error::ApiResult;
use crate::state::AppState;

// --- Klassen ---------------------------------------------------------------

#[tauri::command]
pub async fn class_list(state: State<'_, AppState>) -> ApiResult<Vec<Class>> {
    state.with(|app| class::list(app.conn()))
}

#[tauri::command]
pub async fn class_create(state: State<'_, AppState>, input: ClassInput) -> ApiResult<Class> {
    state.with(|app| class::create(app.conn(), input))
}

#[tauri::command]
pub async fn class_update(
    state: State<'_, AppState>,
    id: Uuid,
    input: ClassInput,
) -> ApiResult<Class> {
    state.with(|app| class::update(app.conn(), id, input))
}

/// Löscht die Klasse samt ihren Schülern.
#[tauri::command]
pub async fn class_delete(state: State<'_, AppState>, id: Uuid) -> ApiResult<()> {
    state.with(|app| class::delete(app.conn(), id))
}

// --- Schüler ---------------------------------------------------------------

/// Die Schüler der Klasse `id`.
#[tauri::command]
pub async fn student_list(state: State<'_, AppState>, id: Uuid) -> ApiResult<Vec<Student>> {
    state.with(|app| student::list(app.conn(), id))
}

/// Legt Schüler in der Klasse `id` an: eine Zeile `Nachname, Vorname` je
/// Schüler. Entweder alle oder keine; Fehler tragen `fields` für `students`.
#[tauri::command]
pub async fn student_add(
    state: State<'_, AppState>,
    id: Uuid,
    text: String,
) -> ApiResult<Vec<Student>> {
    state.with(|app| student::add_many(app.conn(), id, &text))
}

#[tauri::command]
pub async fn student_update(
    state: State<'_, AppState>,
    id: Uuid,
    input: StudentInput,
) -> ApiResult<Student> {
    state.with(|app| student::update(app.conn(), id, input))
}

#[tauri::command]
pub async fn student_delete(state: State<'_, AppState>, id: Uuid) -> ApiResult<()> {
    state.with(|app| student::delete(app.conn(), id))
}

// --- Prüfungen -------------------------------------------------------------

#[tauri::command]
pub async fn exam_list(state: State<'_, AppState>) -> ApiResult<Vec<Exam>> {
    state.with(|app| exam::list(app.conn()))
}

/// Schema und Matrix kommen als Ganzes; Fehler tragen `fields` für `name`,
/// `classes`, `groups` und `grades`.
#[tauri::command]
pub async fn exam_create(state: State<'_, AppState>, input: ExamInput) -> ApiResult<Exam> {
    state.with(|app| exam::create(app.conn(), input))
}

/// Ersetzt Name, Klassen, Schema und Matrix der Prüfung `id`.
#[tauri::command]
pub async fn exam_update(
    state: State<'_, AppState>,
    id: Uuid,
    input: ExamInput,
) -> ApiResult<Exam> {
    state.with(|app| exam::update(app.conn(), id, input))
}

/// Rechnet einen ungespeicherten Entwurf durch (Gruppensummen, Gesamtpunkte,
/// „bis“ der Noten). Prüft nichts und schreibt nichts.
#[tauri::command]
pub async fn exam_preview(input: ExamInput) -> ApiResult<Preview> {
    Ok(exam::preview(&input))
}

#[tauri::command]
pub async fn exam_delete(state: State<'_, AppState>, id: Uuid) -> ApiResult<()> {
    state.with(|app| exam::delete(app.conn(), id))
}

/// Schreibt die Prüfung `id` als `.xlsx` nach `path`. Den Pfad liefert der
/// Speichern-Dialog der Oberfläche.
#[tauri::command]
pub async fn exam_export(state: State<'_, AppState>, id: Uuid, path: String) -> ApiResult<()> {
    state.with(|app| export::write_xlsx(app.conn(), id, std::path::Path::new(&path)))
}

// --- Bewertung -------------------------------------------------------------

/// Die Bewertung aller Schüler der Klasse `class` in der Prüfung `id`:
/// Punkte, Begründungen, Summen und Note — berechnet von core.
#[tauri::command]
pub async fn exam_results(
    state: State<'_, AppState>,
    id: Uuid,
    class: Uuid,
) -> ApiResult<Vec<StudentResult>> {
    state.with(|app| grading::results(app.conn(), id, class))
}

/// Trägt Punkte und Begründung zu einem Prüfkriterium ein und liefert die neu
/// berechnete Bewertung des Schülers. Fehler tragen `fields` für `points`,
/// `note` und `item`.
#[tauri::command]
pub async fn exam_score(
    state: State<'_, AppState>,
    id: Uuid,
    input: ScoreInput,
) -> ApiResult<StudentResult> {
    state.with(|app| grading::set(app.conn(), id, input))
}

// --- Einstellungen ---------------------------------------------------------

#[tauri::command]
pub async fn config_get(state: State<'_, AppState>) -> ApiResult<Config> {
    state.with(|app| Ok(app.config().clone()))
}

/// Prüft und speichert. Bei ungültigen Werten bleibt der alte Stand stehen,
/// und der Fehler trägt `fields` — die Oberfläche zeigt ihn am Feld.
#[tauri::command]
pub async fn config_set(state: State<'_, AppState>, config: Config) -> ApiResult<Config> {
    state.with(|app| {
        app.set_config(config)?;
        Ok(app.config().clone())
    })
}

// --- Über die Anwendung ----------------------------------------------------

/// Wo die Dateien liegen — dieselben Pfade wie `teacher-class paths --json`.
#[derive(Debug, Clone, Serialize, TS)]
pub struct AppInfo {
    pub version: String,
    pub data_dir: String,
    pub config_file: String,
    pub db_file: String,
    pub log_file: String,
}

impl AppInfo {
    fn new(paths: &Paths) -> Self {
        let text = |p: &std::path::Path| p.display().to_string();
        Self {
            version: env!("CARGO_PKG_VERSION").to_string(),
            data_dir: text(paths.data_dir()),
            config_file: text(&paths.config_file()),
            db_file: text(&paths.db_file()),
            log_file: text(&paths.log_file()),
        }
    }
}

#[tauri::command]
pub async fn app_info(state: State<'_, AppState>) -> ApiResult<AppInfo> {
    state.with(|app| Ok(AppInfo::new(app.paths())))
}

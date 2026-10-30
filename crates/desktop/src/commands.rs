//! Die Befehle, die das Webview aufrufen kann.
//!
//! **Jeder Befehl ist dünn:** Argumente annehmen, eine Funktion in
//! `starter_core` rufen, das Ergebnis zurückgeben. Prüfung und SQL stehen in
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
use starter_core::note::{self, Note, NoteInput, Order};
use starter_core::{Config, Paths};
use tauri::State;
use ts_rs::TS;
use uuid::Uuid;

use crate::error::ApiResult;
use crate::state::AppState;

// --- Notizen ---------------------------------------------------------------

#[tauri::command]
pub async fn note_list(state: State<'_, AppState>, order: Order) -> ApiResult<Vec<Note>> {
    state.with(|app| note::list(app.conn(), order))
}

#[tauri::command]
pub async fn note_get(state: State<'_, AppState>, id: Uuid) -> ApiResult<Note> {
    state.with(|app| note::get(app.conn(), id))
}

#[tauri::command]
pub async fn note_create(state: State<'_, AppState>, input: NoteInput) -> ApiResult<Note> {
    state.with(|app| note::create(app.conn(), input))
}

#[tauri::command]
pub async fn note_update(
    state: State<'_, AppState>,
    id: Uuid,
    input: NoteInput,
) -> ApiResult<Note> {
    state.with(|app| note::update(app.conn(), id, input))
}

#[tauri::command]
pub async fn note_delete(state: State<'_, AppState>, id: Uuid) -> ApiResult<()> {
    state.with(|app| note::delete(app.conn(), id))
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

/// Wo die Dateien liegen — dieselben Pfade wie `starter paths --json`.
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

//! Der Zustand, den alle Befehle teilen.
//!
//! Eine `rusqlite::Connection` darf zwischen Fäden wandern, aber nicht von
//! zweien gleichzeitig benutzt werden. Tauri ruft `async`-Befehle auf einem
//! Fadenpool auf — deshalb liegt die [`App`] hinter einem `Mutex`.
//!
//! Die Sperre wird nie über ein `.await` hinweg gehalten. Die Befehle in
//! `commands.rs` haben kein `.await`, und so soll es bleiben: Was lange dauert,
//! gehört in `tauri::async_runtime::spawn_blocking`, siehe docs/RECIPES.md.

use std::sync::Mutex;

use starter_core::App;

use crate::error::{ApiError, ApiResult};

pub struct AppState {
    inner: Mutex<Result<App, ApiError>>,
}

impl AppState {
    pub fn new(app: App) -> Self {
        Self {
            inner: Mutex::new(Ok(app)),
        }
    }

    /// Die Anwendung ließ sich nicht öffnen — etwa weil `config.toml` einen
    /// Tippfehler hat.
    ///
    /// Das Fenster geht trotzdem auf, und jeder Befehl liefert diesen Fehler.
    /// So sieht der Benutzer eine Meldung statt eines Programms, das beim
    /// Doppelklick kommentarlos verschwindet.
    pub fn failed(err: ApiError) -> Self {
        Self {
            inner: Mutex::new(Err(err)),
        }
    }

    /// Führt `f` mit der geöffneten Anwendung aus.
    pub fn with<T>(&self, f: impl FnOnce(&mut App) -> starter_core::Result<T>) -> ApiResult<T> {
        // Eine vergiftete Sperre heißt: Ein früherer Befehl ist mitten in der
        // Arbeit abgestürzt. Die Datenbank selbst ist dank Transaktionen
        // heil, also weiterarbeiten statt jeden weiteren Befehl scheitern zu
        // lassen.
        let mut guard = self.inner.lock().unwrap_or_else(|e| e.into_inner());
        match guard.as_mut() {
            Ok(app) => f(app).map_err(ApiError::from),
            Err(err) => Err(err.clone()),
        }
    }
}

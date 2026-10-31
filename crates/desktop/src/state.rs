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

use teacher_class_core::{App, Paths};

use crate::error::{ApiError, ApiResult};

pub struct AppState {
    inner: Mutex<Inner>,
}

struct Inner {
    app: Result<App, ApiError>,
    /// Die Pfade der Plattform ohne gewählten Datenbankort. Damit lässt sich
    /// der Ort auch dann wählen, wenn die Anwendung gar nicht geöffnet ist,
    /// weil am bisherigen Ort keine Datenbank liegt.
    base: Option<Paths>,
}

impl AppState {
    pub fn new(app: App) -> Self {
        let base = app.paths().clone().with_db_file(None);
        Self {
            inner: Mutex::new(Inner {
                app: Ok(app),
                base: Some(base),
            }),
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
            inner: Mutex::new(Inner {
                app: Err(err),
                base: None,
            }),
        }
    }

    /// Wie [`AppState::failed`], aber mit den Pfaden — damit die Oberfläche
    /// einen anderen Speicherort für die Datenbank wählen kann
    /// ([`AppState::switch`]).
    pub fn failed_with_paths(base: Paths, err: ApiError) -> Self {
        Self {
            inner: Mutex::new(Inner {
                app: Err(err),
                base: Some(base),
            }),
        }
    }

    /// Führt `f` mit der geöffneten Anwendung aus.
    pub fn with<T>(
        &self,
        f: impl FnOnce(&mut App) -> teacher_class_core::Result<T>,
    ) -> ApiResult<T> {
        let mut guard = self.lock();
        match guard.app.as_mut() {
            Ok(app) => f(app).map_err(ApiError::from),
            Err(err) => Err(err.clone()),
        }
    }

    /// Öffnet die Anwendung an einem anderen Datenbankort und ersetzt den
    /// Zustand. Scheitert `open`, bleibt alles, wie es war.
    pub fn switch(
        &self,
        open: impl FnOnce(&Paths) -> teacher_class_core::Result<App>,
    ) -> ApiResult<()> {
        let mut guard = self.lock();
        let Some(base) = guard.base.clone() else {
            return Err(ApiError::internal(
                "Die Pfade der Anwendung sind unbekannt — sie ließ sich nicht öffnen.",
            ));
        };
        let app = open(&base).map_err(ApiError::from)?;
        guard.app = Ok(app);
        Ok(())
    }

    fn lock(&self) -> std::sync::MutexGuard<'_, Inner> {
        // Eine vergiftete Sperre heißt: Ein früherer Befehl ist mitten in der
        // Arbeit abgestürzt. Die Datenbank selbst ist dank Transaktionen
        // heil, also weiterarbeiten statt jeden weiteren Befehl scheitern zu
        // lassen.
        self.inner.lock().unwrap_or_else(|e| e.into_inner())
    }
}

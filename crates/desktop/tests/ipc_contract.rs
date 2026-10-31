//! Die IPC-Grenze zwischen Webview und Rust.
//!
//! Drei Sorten Prüfung:
//!
//! 1. **Echte Aufrufe** über `tauri::test::MockRuntime` — ohne Fenster, aber
//!    mit derselben Deserialisierung der Argumente wie im Webview. Ein
//!    Argument, das in Rust `note_id` heißt und im JSON `id`, fällt hier auf
//!    und nicht erst beim Klicken.
//! 2. **Die Befehlsliste** in `lib.rs` gegen die in `ui/src/api.ts`. Ein
//!    Befehl, der nur auf einer Seite steht, ist entweder toter Code oder ein
//!    Aufruf, der zur Laufzeit mit „command not found" scheitert.
//! 3. **Die Sicherheitsrichtlinie** in `tauri.conf.json` — so eng wie möglich,
//!    so weit wie Mermaid es braucht.
//!
//! Die fachlichen Regeln stehen in den Tests von `teacher-class-core`. Hier geht es
//! nur ums Durchreichen.

use serde_json::{Value, json};
use tauri::ipc::{CallbackFn, InvokeBody};
use tauri::test::{INVOKE_KEY, MockRuntime, mock_builder, mock_context, noop_assets};
use tauri::webview::InvokeRequest;
use tauri::{WebviewWindow, WebviewWindowBuilder};
use teacher_class_core::{App, Paths};
use teacher_class_desktop::{AppState, configure};

struct Ctx {
    _dir: tempfile::TempDir,
    // Die App muss leben, solange das Fenster benutzt wird.
    _app: tauri::App<MockRuntime>,
    window: WebviewWindow<MockRuntime>,
}

impl Ctx {
    fn new() -> Self {
        let dir = tempfile::tempdir().unwrap();
        let core = App::open(Paths::under(dir.path())).unwrap();
        Self::with_state(dir, AppState::new(core))
    }

    fn with_state(dir: tempfile::TempDir, state: AppState) -> Self {
        let app = configure(mock_builder(), state)
            .build(mock_context(noop_assets()))
            .expect("Tauri-Attrappe baut");
        let window = WebviewWindowBuilder::new(&app, "main", Default::default())
            .build()
            .expect("Fenster der Attrappe");
        Self {
            _dir: dir,
            _app: app,
            window,
        }
    }

    /// Ruft einen Befehl so auf, wie `invoke(cmd, args)` im Webview es tut.
    fn invoke(&self, cmd: &str, args: Value) -> Result<Value, Value> {
        tauri::test::get_ipc_response(
            &self.window,
            InvokeRequest {
                cmd: cmd.into(),
                callback: CallbackFn(0),
                error: CallbackFn(1),
                // Die Adresse der eigenen Oberfläche — vom Fenster erfragt,
                // nicht hingeschrieben. Sie lautet auf Windows anders als auf
                // macOS und Linux (`http://tauri.localhost` statt
                // `tauri://localhost`), und unter einer fremden Adresse
                // verweigert Tauri jeden Befehl. Wer sie hier festnagelt, baut
                // einen Test, der auf genau einer Plattform läuft.
                url: self.window.url().expect("Adresse des Fensters"),
                body: InvokeBody::Json(args),
                headers: Default::default(),
                invoke_key: INVOKE_KEY.to_string(),
            },
        )
        .map(|body| body.deserialize::<Value>().expect("Antwort ist JSON"))
    }
}

#[test]
fn legt_an_listet_aendert_und_loescht() {
    let ctx = Ctx::new();

    let created = ctx
        .invoke(
            "note_create",
            json!({ "input": { "title": "Einkauf", "body": "Milch" } }),
        )
        .unwrap();
    assert_eq!(created["title"], "Einkauf");
    let id = created["id"].as_str().unwrap().to_string();

    let list = ctx
        .invoke("note_list", json!({ "order": "recent" }))
        .unwrap();
    assert_eq!(list.as_array().unwrap().len(), 1);

    let updated = ctx
        .invoke(
            "note_update",
            json!({ "id": id, "input": { "title": "Großeinkauf", "body": "" } }),
        )
        .unwrap();
    assert_eq!(updated["title"], "Großeinkauf");

    ctx.invoke("note_delete", json!({ "id": id })).unwrap();
    let list = ctx
        .invoke("note_list", json!({ "order": "title" }))
        .unwrap();
    assert!(list.as_array().unwrap().is_empty());
}

#[test]
fn eingabefehler_kommen_mit_feldern_an() {
    let ctx = Ctx::new();
    let err = ctx
        .invoke(
            "note_create",
            json!({ "input": { "title": "  ", "body": "" } }),
        )
        .unwrap_err();

    assert_eq!(err["code"], "validation_failed");
    assert_eq!(err["fields"][0]["field"], "title");
}

#[test]
fn unbekannte_kennung_ist_not_found() {
    let ctx = Ctx::new();
    let err = ctx
        .invoke("note_get", json!({ "id": uuid::Uuid::now_v7() }))
        .unwrap_err();
    assert_eq!(err["code"], "not_found");
    assert!(err.get("fields").is_none(), "kein Eingabefehler");
}

#[test]
fn ungueltige_einstellungen_bleiben_ungespeichert() {
    let ctx = Ctx::new();
    let before = ctx.invoke("config_get", json!({})).unwrap();

    let mut broken = before.clone();
    broken["accent"] = json!("blau");
    let err = ctx
        .invoke("config_set", json!({ "config": broken }))
        .unwrap_err();
    assert_eq!(err["fields"][0]["field"], "accent");

    assert_eq!(ctx.invoke("config_get", json!({})).unwrap(), before);
}

#[test]
fn ein_fehler_beim_start_erreicht_jeden_befehl() {
    // Die Anwendung ließ sich nicht öffnen — das Fenster soll trotzdem eine
    // Meldung zeigen können, statt kommentarlos zu verschwinden.
    let dir = tempfile::tempdir().unwrap();
    let state = AppState::failed(teacher_class_desktop::ApiError::internal("kaputt"));
    let ctx = Ctx::with_state(dir, state);

    let err = ctx
        .invoke("note_list", json!({ "order": "recent" }))
        .unwrap_err();
    assert_eq!(err["message"], "kaputt");
}

#[test]
fn falscher_argumentname_wird_abgelehnt() {
    // Der häufigste Tauri-Fehler: Rust erwartet `input`, das Webview schickt
    // etwas anderes. Der Aufruf muss scheitern, nicht still leer anlegen.
    let ctx = Ctx::new();
    let err = ctx
        .invoke(
            "note_create",
            json!({ "note": { "title": "x", "body": "" } }),
        )
        .unwrap_err();
    assert!(
        err.to_string().contains("input"),
        "Meldung nennt das fehlende Argument: {err}"
    );
}

// --- Sicherheitsrichtlinie -------------------------------------------------

#[test]
fn csp_laesst_inline_stile_von_diagrammen_zu() {
    // Tauri hängt beim Bündeln eine Nonce an style-src. Dann ignoriert der
    // Browser 'unsafe-inline', und Mermaid zeichnet schwarze Flächen — nur im
    // fertigen Bündel, nie in `just dev`. Siehe CLAUDE.md, Punkt 7.
    let conf: Value = serde_json::from_str(include_str!("../tauri.conf.json")).unwrap();
    let security = &conf["app"]["security"];

    let disabled = security["dangerousDisableAssetCspModification"]
        .as_array()
        .expect("dangerousDisableAssetCspModification als Liste");
    assert!(disabled.iter().any(|d| d == "style-src"));
    assert!(
        !disabled.iter().any(|d| d == "script-src"),
        "die Nonce für Skripte bleibt"
    );

    let csp = security["csp"].as_object().expect("csp als Objekt");
    assert!(
        csp["style-src"]
            .as_str()
            .unwrap()
            .contains("'unsafe-inline'")
    );
    for (directive, value) in csp {
        let value = value.as_str().unwrap();
        assert!(
            !value.contains("'unsafe-eval'") && !value.split(' ').any(|v| v == "*"),
            "{directive} öffnet die CSP zu weit: {value}"
        );
    }
}

#[test]
fn buendelkennung_ist_gueltig() {
    // Tauri und macOS erlauben nur Buchstaben, Ziffern, "-" und ".". Ein
    // Unterstrich — etwa aus einem Kurznamen mit Bindestrich — fällt sonst
    // erst bei `tauri build` auf.
    let conf: Value = serde_json::from_str(include_str!("../tauri.conf.json")).unwrap();
    let id = conf["identifier"]
        .as_str()
        .expect("identifier in tauri.conf.json");

    assert!(
        id.split('.').count() >= 2
            && id.split('.').all(|part| !part.is_empty())
            && id
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '.'),
        "ungültige Bündelkennung {id:?} — nur A–Z, a–z, 0–9, \"-\" und \".\""
    );
}

// --- Befehlsliste Rust ↔ TypeScript ---------------------------------------

#[test]
fn jeder_befehl_steht_auf_beiden_seiten() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .unwrap()
        .to_path_buf();

    let rust =
        rust_commands(&std::fs::read_to_string(root.join("crates/desktop/src/lib.rs")).unwrap());
    let ts = ts_commands(&std::fs::read_to_string(root.join("ui/src/api.ts")).unwrap());

    let only_rust: Vec<_> = rust.iter().filter(|c| !ts.contains(c)).collect();
    let only_ts: Vec<_> = ts.iter().filter(|c| !rust.contains(c)).collect();

    assert!(
        only_rust.is_empty() && only_ts.is_empty(),
        "Befehlslisten passen nicht zusammen.\n\
         Nur in lib.rs (generate_handler!): {only_rust:?}\n\
         Nur in ui/src/api.ts:              {only_ts:?}"
    );
    assert!(
        !rust.is_empty(),
        "keine Befehle in generate_handler! gefunden"
    );
}

/// `commands::note_list,` innerhalb von `generate_handler![ … ]`.
fn rust_commands(lib_rs: &str) -> Vec<String> {
    let start = lib_rs
        .find("generate_handler![")
        .expect("generate_handler! in lib.rs");
    let end = start + lib_rs[start..].find(']').expect("schließende Klammer");
    let mut out: Vec<String> = lib_rs[start + "generate_handler![".len()..end]
        .split(',')
        .map(|s| s.trim())
        .filter(|s| !s.is_empty() && !s.starts_with("//"))
        .map(|s| s.rsplit("::").next().unwrap().to_string())
        .collect();
    out.sort();
    out
}

/// `call<Note[]>("note_list", …)` in api.ts — ein Aufruf je Zeile.
fn ts_commands(api_ts: &str) -> Vec<String> {
    let mut out: Vec<String> = api_ts
        .lines()
        .filter(|line| !line.trim_start().starts_with("//"))
        .filter_map(|line| {
            let rest = &line[line.find("call<")?..];
            let open = rest.find(">(\"")? + 3;
            let close = open + rest[open..].find('"')?;
            Some(rest[open..close].to_string())
        })
        .collect();
    out.sort();
    out.dedup();
    out
}

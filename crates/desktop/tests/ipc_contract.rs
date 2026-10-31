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
            "class_create",
            json!({ "input": { "name": "7a", "year": 2025 } }),
        )
        .unwrap();
    assert_eq!(created["name"], "7a");
    let id = created["id"].as_str().unwrap().to_string();

    let students = ctx
        .invoke(
            "student_add",
            json!({ "id": id, "text": "Muster, Max\nBeispiel, Eva" }),
        )
        .unwrap();
    assert_eq!(students.as_array().unwrap().len(), 2);
    let student_id = students[0]["id"].as_str().unwrap().to_string();

    let list = ctx.invoke("student_list", json!({ "id": id })).unwrap();
    assert_eq!(list[0]["last_name"], "Beispiel", "alphabetisch sortiert");

    let renamed = ctx
        .invoke(
            "student_update",
            json!({ "id": student_id, "input": { "last_name": "Muster", "first_name": "Moritz" } }),
        )
        .unwrap();
    assert_eq!(renamed["first_name"], "Moritz");
    ctx.invoke("student_delete", json!({ "id": student_id }))
        .unwrap();

    let updated = ctx
        .invoke(
            "class_update",
            json!({ "id": id, "input": { "name": "7b", "year": 2025 } }),
        )
        .unwrap();
    assert_eq!(updated["name"], "7b");
    assert_eq!(updated["student_count"], 1);

    ctx.invoke("class_delete", json!({ "id": id })).unwrap();
    let list = ctx.invoke("class_list", json!({})).unwrap();
    assert!(list.as_array().unwrap().is_empty());
}

fn pruefung_json(class_id: &Value) -> Value {
    json!({
        "name": "Klassenarbeit",
        "class_ids": [class_id],
        "groups": [{ "name": "Aufgabe 1", "items": [
            { "name": "a", "max_points": 4.0 }, { "name": "b", "max_points": 2.5 }
        ]}],
        "grades": [
            { "name": "1", "from_points": 5.0 },
            { "name": "6", "from_points": 0.0 }
        ]
    })
}

#[test]
fn pruefungen_anlegen_aendern_exportieren_loeschen() {
    let ctx = Ctx::new();
    let class = ctx
        .invoke(
            "class_create",
            json!({ "input": { "name": "7a", "year": 2025 } }),
        )
        .unwrap();
    ctx.invoke(
        "student_add",
        json!({ "id": class["id"], "text": "Muster, Max" }),
    )
    .unwrap();

    let created = ctx
        .invoke(
            "exam_create",
            json!({ "input": pruefung_json(&class["id"]) }),
        )
        .unwrap();
    assert_eq!(created["max_points"], 6.5);
    // „bis“ wird in Rust berechnet und kommt fertig an.
    assert_eq!(created["grades"][0]["name"], "6");
    assert_eq!(created["grades"][0]["to_points"], 4.5);
    let id = created["id"].as_str().unwrap().to_string();

    let mut changed = pruefung_json(&class["id"]);
    changed["name"] = json!("Test");
    let updated = ctx
        .invoke("exam_update", json!({ "id": id, "input": changed }))
        .unwrap();
    assert_eq!(updated["name"], "Test");
    assert_eq!(
        ctx.invoke("exam_list", json!({}))
            .unwrap()
            .as_array()
            .unwrap()
            .len(),
        1
    );

    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("pruefung.xlsx");
    ctx.invoke(
        "exam_export",
        json!({ "id": id, "path": path.to_str().unwrap() }),
    )
    .unwrap();
    assert_eq!(&std::fs::read(&path).unwrap()[..2], b"PK");

    let err = ctx
        .invoke(
            "exam_export",
            json!({ "id": id, "path": dir.path().join("x.txt").to_str().unwrap() }),
        )
        .unwrap_err();
    assert_eq!(err["fields"][0]["field"], "path");

    ctx.invoke("exam_delete", json!({ "id": id })).unwrap();
    assert!(
        ctx.invoke("exam_list", json!({}))
            .unwrap()
            .as_array()
            .unwrap()
            .is_empty()
    );
}

#[test]
fn pruefungsfehler_kommen_am_feld_an() {
    let ctx = Ctx::new();
    let class = ctx
        .invoke(
            "class_create",
            json!({ "input": { "name": "7a", "year": 2025 } }),
        )
        .unwrap();
    let mut input = pruefung_json(&class["id"]);
    input["grades"] = json!([{ "name": "1", "from_points": 2.0 }]);
    let err = ctx
        .invoke("exam_create", json!({ "input": input }))
        .unwrap_err();
    assert_eq!(err["fields"][0]["field"], "grades");
}

#[test]
fn eingabefehler_kommen_mit_feldern_an() {
    let ctx = Ctx::new();
    let err = ctx
        .invoke(
            "class_create",
            json!({ "input": { "name": "7 a", "year": 25 } }),
        )
        .unwrap_err();

    assert_eq!(err["code"], "validation_failed");
    assert_eq!(err["fields"][0]["field"], "name");
    assert_eq!(err["fields"][1]["field"], "year");
}

#[test]
fn schuelerfehler_tragen_die_zeilennummer_am_feld_students() {
    let ctx = Ctx::new();
    let class = ctx
        .invoke(
            "class_create",
            json!({ "input": { "name": "7a", "year": 2025 } }),
        )
        .unwrap();
    let err = ctx
        .invoke(
            "student_add",
            json!({ "id": class["id"], "text": "Gut, Gerd\nohne Komma" }),
        )
        .unwrap_err();
    assert_eq!(err["fields"][0]["field"], "students");
    assert!(
        err["fields"][0]["message"]
            .as_str()
            .unwrap()
            .contains("Zeile 2")
    );
}

#[test]
fn unbekannte_kennung_ist_not_found() {
    let ctx = Ctx::new();
    let err = ctx
        .invoke("student_list", json!({ "id": uuid::Uuid::now_v7() }))
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

    let err = ctx.invoke("class_list", json!({})).unwrap_err();
    assert_eq!(err["message"], "kaputt");
}

#[test]
fn fehlt_die_datenbank_am_gewaehlten_ort_laesst_sich_ein_anderer_waehlen() {
    // Der Weg aus der Oberfläche: Start meldet `database_missing`, der
    // Benutzer legt eine neue Datenbank an, danach geht alles wieder.
    let dir = tempfile::tempdir().unwrap();
    let base = Paths::under(dir.path());
    let weg = dir.path().join("weg.sqlite3");
    drop(App::create_database(&base, &weg).unwrap());
    std::fs::remove_file(&weg).unwrap();

    let err = App::open(base.clone()).err().unwrap();
    let state = AppState::failed_with_paths(base, err.into());
    let ctx = Ctx::with_state(dir, state);

    let err = ctx.invoke("class_list", json!({})).unwrap_err();
    assert_eq!(err["code"], "database_missing");

    let neu = ctx._dir.path().join("neu.sqlite3");
    ctx.invoke("db_create", json!({ "path": neu })).unwrap();

    assert_eq!(ctx.invoke("class_list", json!({})).unwrap(), json!([]));
    let info = ctx.invoke("app_info", json!({})).unwrap();
    assert_eq!(info["db_file"], json!(neu.display().to_string()));
    assert_eq!(
        ctx.invoke("config_get", json!({})).unwrap()["db_file"],
        json!(neu.display().to_string())
    );
}

#[test]
fn db_use_lehnt_eine_fehlende_datei_am_feld_ab() {
    let ctx = Ctx::new();
    let nix = ctx._dir.path().join("nix.sqlite3");

    let err = ctx.invoke("db_use", json!({ "path": nix })).unwrap_err();

    assert_eq!(err["code"], "validation_failed");
    assert_eq!(err["fields"][0]["field"], "db_file");
    // Die laufende Datenbank bleibt benutzbar.
    assert!(ctx.invoke("class_list", json!({})).is_ok());
}

#[test]
fn db_reset_geht_zurueck_zum_standardort() {
    let ctx = Ctx::new();
    let ziel = ctx._dir.path().join("x.sqlite3");
    ctx.invoke("db_create", json!({ "path": ziel })).unwrap();

    ctx.invoke("db_reset", json!({})).unwrap();

    assert_eq!(
        ctx.invoke("config_get", json!({})).unwrap()["db_file"],
        Value::Null
    );
}

#[test]
fn config_set_aendert_den_speicherort_nicht() {
    let ctx = Ctx::new();
    let mut config = ctx.invoke("config_get", json!({})).unwrap();
    config["db_file"] = json!(ctx._dir.path().join("y.sqlite3"));

    let err = ctx
        .invoke("config_set", json!({ "config": config }))
        .unwrap_err();

    assert_eq!(err["fields"][0]["field"], "db_file");
}

#[test]
fn falscher_argumentname_wird_abgelehnt() {
    // Der häufigste Tauri-Fehler: Rust erwartet `input`, das Webview schickt
    // etwas anderes. Der Aufruf muss scheitern, nicht still leer anlegen.
    let ctx = Ctx::new();
    let err = ctx
        .invoke(
            "class_create",
            json!({ "class": { "name": "7a", "year": 2025 } }),
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

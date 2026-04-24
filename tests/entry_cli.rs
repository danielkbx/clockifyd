mod support;

use std::fs;

use support::{bin, stderr, stdout, MockResponse, TestServer};

#[test]
fn help_entry_works() {
    let output = bin().args(["help", "entry"]).output().unwrap();

    assert!(output.status.success());
    let stdout = stdout(&output);
    assert!(stdout.contains("cfd entry list"));
    assert!(stdout.contains("--text <value>"));
}

#[test]
fn entry_text_help_works() {
    let output = bin().args(["entry", "text", "help"]).output().unwrap();

    assert!(output.status.success());
    assert!(stdout(&output).contains("cfd entry text list"));
}

#[test]
fn entry_list_rejects_invalid_time_argument_before_network() {
    let (_dir, config_path) = support::temp_config_path();

    let output = bin()
        .args(["entry", "list", "--start", "not-a-date"])
        .env("CFD_CONFIG", &config_path)
        .env("CLOCKIFY_API_KEY", "dummy-key")
        .env("CFD_WORKSPACE", "w1")
        .output()
        .unwrap();

    assert!(!output.status.success());
    assert!(stderr(&output).contains("invalid start"));
}

#[test]
fn entry_text_list_requires_project_context() {
    let (_dir, config_path) = support::temp_config_path();

    let output = bin()
        .args(["entry", "text", "list"])
        .env("CFD_CONFIG", &config_path)
        .env("CLOCKIFY_API_KEY", "dummy-key")
        .env("CFD_WORKSPACE", "w1")
        .output()
        .unwrap();

    assert!(!output.status.success());
    assert!(stderr(&output).contains("cfd config set project <id>"));
}

#[test]
fn entry_text_list_uses_config_project_default_and_no_meta() {
    let (_dir, config_path) = support::temp_config_path();
    fs::write(&config_path, "{\n  \"project\": \"p1\"\n}\n").unwrap();
    let server = TestServer::spawn(vec![
        MockResponse::ok(r#"{"id":"u1","name":"Ada","email":"ada@example.com"}"#),
        MockResponse::ok(
            r#"[{"id":"e1","workspaceId":"w1","userId":"u1","projectId":"p1","description":"  Focus work  ","timeInterval":{"start":"2026-04-23T10:00:00Z","end":"2026-04-23T11:00:00Z","duration":"PT1H"}},{"id":"e2","workspaceId":"w1","userId":"u1","projectId":"p1","description":"Focus work","timeInterval":{"start":"2026-04-24T10:00:00Z","end":"2026-04-24T11:00:00Z","duration":"PT1H"}}]"#,
        ),
    ]);

    let output = bin()
        .args(["entry", "text", "list", "--no-meta"])
        .env("CFD_CONFIG", &config_path)
        .env("CLOCKIFY_API_KEY", "secret")
        .env("CFD_WORKSPACE", "w1")
        .env("CFD_BASE_URL", server.base_url())
        .output()
        .unwrap();
    assert!(output.status.success());
    assert_eq!(stdout(&output), "Focus work\n");

    let requests = server.requests();
    assert!(requests[1].path.contains("project=p1"));
}

#[test]
fn entry_text_list_supports_columns_and_validation() {
    let (_dir, config_path) = support::temp_config_path();
    fs::write(&config_path, "{\n  \"project\": \"p1\"\n}\n").unwrap();
    let server = TestServer::spawn(vec![
        MockResponse::ok(r#"{"id":"u1","name":"Ada","email":"ada@example.com"}"#),
        MockResponse::ok(
            r#"[{"id":"e1","workspaceId":"w1","userId":"u1","projectId":"p1","description":"Focus work","timeInterval":{"start":"2026-04-24T10:00:00Z","end":"2026-04-24T11:00:00Z","duration":"PT1H"}}]"#,
        ),
    ]);

    let columns = bin()
        .args(["entry", "text", "list", "--columns", "text,lastUsed,count"])
        .env("CFD_CONFIG", &config_path)
        .env("CLOCKIFY_API_KEY", "secret")
        .env("CFD_WORKSPACE", "w1")
        .env("CFD_BASE_URL", server.base_url())
        .output()
        .unwrap();
    assert!(columns.status.success());
    assert_eq!(stdout(&columns), "Focus work\t2026-04-24T10:00:00Z\t1\n");

    let missing = bin()
        .args(["entry", "text", "list", "--columns"])
        .env("CFD_CONFIG", &config_path)
        .env("CLOCKIFY_API_KEY", "secret")
        .env("CFD_WORKSPACE", "w1")
        .output()
        .unwrap();
    assert!(!missing.status.success());
    assert!(
        stderr(&missing).contains("usage: cfd entry text list --columns <text,lastUsed,count,...>")
    );

    let conflict = bin()
        .args([
            "entry",
            "text",
            "list",
            "--columns",
            "text,lastUsed",
            "--format",
            "json",
        ])
        .env("CFD_CONFIG", &config_path)
        .env("CLOCKIFY_API_KEY", "secret")
        .env("CFD_WORKSPACE", "w1")
        .output()
        .unwrap();
    assert!(!conflict.status.success());
    assert!(
        stderr(&conflict).contains("use either --columns <list> or --format <text|json>, not both")
    );
}

#[test]
fn entry_list_forwards_text_filter_and_resolves_date_keywords() {
    let server = TestServer::spawn(vec![
        MockResponse::ok(r#"{"id":"u1","name":"Ada","email":"ada@example.com"}"#),
        MockResponse::ok("[]"),
        MockResponse::ok(r#"{"id":"u1","name":"Ada","email":"ada@example.com"}"#),
        MockResponse::ok("[]"),
        MockResponse::ok(r#"{"id":"u1","name":"Ada","email":"ada@example.com"}"#),
        MockResponse::ok("[]"),
    ]);

    let today = bin()
        .args([
            "entry", "list", "--start", "today", "--end", "today", "--text", "focus", "--format",
            "json",
        ])
        .env("CLOCKIFY_API_KEY", "secret")
        .env("CFD_WORKSPACE", "w1")
        .env("CFD_BASE_URL", server.base_url())
        .output()
        .unwrap();
    assert!(today.status.success());
    assert!(stdout(&today).contains("["));

    let yesterday = bin()
        .args([
            "entry",
            "list",
            "--start",
            "yesterday",
            "--end",
            "yesterday",
        ])
        .env("CLOCKIFY_API_KEY", "secret")
        .env("CFD_WORKSPACE", "w1")
        .env("CFD_BASE_URL", server.base_url())
        .output()
        .unwrap();
    assert!(yesterday.status.success());
    assert_eq!(stdout(&yesterday), "\n");

    let columns = bin()
        .args([
            "entry",
            "list",
            "--start",
            "today",
            "--end",
            "today",
            "--columns",
            "start,end",
        ])
        .env("CLOCKIFY_API_KEY", "secret")
        .env("CFD_WORKSPACE", "w1")
        .env("CFD_BASE_URL", server.base_url())
        .output()
        .unwrap();
    assert!(columns.status.success());
    assert_eq!(stdout(&columns), "\n");

    let requests = server.requests();
    assert!(requests[1].path.contains("description=focus"));
    assert!(requests[1].path.contains("start=20"));
    assert!(requests[1].path.contains("end=20"));
    assert!(!requests[1].path.contains("today"));
    assert!(requests[3].path.contains("start=20"));
    assert!(requests[3].path.contains("end=20"));
    assert!(!requests[3].path.contains("yesterday"));
    assert!(requests[5].path.contains("start=20"));
}

#[test]
fn entry_columns_requires_value_and_rejects_format_combo() {
    let missing = bin()
        .args(["entry", "list", "--columns"])
        .env("CLOCKIFY_API_KEY", "secret")
        .env("CFD_WORKSPACE", "w1")
        .output()
        .unwrap();
    assert!(!missing.status.success());
    assert!(stderr(&missing)
        .contains("--columns <id,start,end,duration,description,projectId,projectName,...>"));

    let conflict = bin()
        .args([
            "entry",
            "list",
            "--columns",
            "start,end",
            "--format",
            "json",
        ])
        .env("CLOCKIFY_API_KEY", "secret")
        .env("CFD_WORKSPACE", "w1")
        .output()
        .unwrap();
    assert!(!conflict.status.success());
    assert!(
        stderr(&conflict).contains("use either --columns <list> or --format <text|json>, not both")
    );
}

#[test]
fn entry_get_text_includes_project_name() {
    let server = TestServer::spawn(vec![
        MockResponse::ok(
            r#"{"id":"e1","workspaceId":"w1","userId":"u1","projectId":"p1","description":"Focus","timeInterval":{"start":"2026-04-23T09:00:00Z","end":"2026-04-23T10:00:00Z","duration":"PT1H"}}"#,
        ),
        MockResponse::ok(r#"{"id":"p1","name":"Project One","workspaceId":"w1"}"#),
    ]);

    let output = bin()
        .args(["entry", "get", "e1"])
        .env("CLOCKIFY_API_KEY", "secret")
        .env("CFD_WORKSPACE", "w1")
        .env("CFD_BASE_URL", server.base_url())
        .output()
        .unwrap();

    assert!(output.status.success());
    let text = stdout(&output);
    assert!(text.contains("projectId: p1\n"));
    assert!(text.contains("project: Project One\n"));
}

#[test]
fn entry_list_text_includes_project_names() {
    let server = TestServer::spawn(vec![
        MockResponse::ok(r#"{"id":"u1","name":"Ada","email":"ada@example.com"}"#),
        MockResponse::ok(
            r#"[{"id":"e1","workspaceId":"w1","userId":"u1","projectId":"p1","description":"Focus","timeInterval":{"start":"2026-04-23T09:00:00Z","end":"2026-04-23T10:00:00Z","duration":"PT1H"}}]"#,
        ),
        MockResponse::ok(r#"[{"id":"p1","name":"Project One","workspaceId":"w1"}]"#),
    ]);

    let output = bin()
        .args(["entry", "list", "--start", "2026-04-23T09:00:00Z"])
        .env("CLOCKIFY_API_KEY", "secret")
        .env("CFD_WORKSPACE", "w1")
        .env("CFD_BASE_URL", server.base_url())
        .output()
        .unwrap();

    assert!(output.status.success());
    let text = stdout(&output);
    assert!(text.contains("projectId: p1\n"));
    assert!(text.contains("project: Project One\n"));
}

#[test]
fn entry_columns_support_project_id_and_name() {
    let server = TestServer::spawn(vec![
        MockResponse::ok(r#"{"id":"u1","name":"Ada","email":"ada@example.com"}"#),
        MockResponse::ok(
            r#"[{"id":"e1","workspaceId":"w1","userId":"u1","projectId":"p1","description":"Focus","timeInterval":{"start":"2026-04-23T09:00:00Z","end":"2026-04-23T10:00:00Z","duration":"PT1H"}}]"#,
        ),
        MockResponse::ok(r#"[{"id":"p1","name":"Project One","workspaceId":"w1"}]"#),
    ]);

    let output = bin()
        .args([
            "entry",
            "list",
            "--start",
            "2026-04-23T09:00:00Z",
            "--columns",
            "start,duration,projectId,projectName",
        ])
        .env("CLOCKIFY_API_KEY", "secret")
        .env("CFD_WORKSPACE", "w1")
        .env("CFD_BASE_URL", server.base_url())
        .output()
        .unwrap();

    assert!(output.status.success());
    assert_eq!(
        stdout(&output),
        "2026-04-23T09:00:00Z\t1h\tp1\tProject One\n"
    );
}

#[test]
fn entry_add_no_rounding_overrides_config_and_prints_id_only() {
    let (_dir, config_path) = support::temp_config_path();
    fs::write(&config_path, "{\n  \"rounding\": \"15m\"\n}\n").unwrap();
    let server = TestServer::spawn(vec![
        MockResponse::ok(r#"{"id":"u1","name":"Ada","email":"ada@example.com"}"#),
        MockResponse::ok("[]"),
        MockResponse::ok(
            r#"{"id":"e1","workspaceId":"w1","userId":"u1","description":"Focus","timeInterval":{"start":"2026-04-23T09:07:00+00:00","end":"2026-04-23T09:40:00+00:00","duration":"PT33M"}}"#,
        ),
    ]);

    let output = bin()
        .args([
            "entry",
            "add",
            "--start",
            "2026-04-23T09:07:00Z",
            "--end",
            "2026-04-23T09:40:00Z",
            "--description",
            "Focus",
            "--no-rounding",
        ])
        .env("CFD_CONFIG", &config_path)
        .env("CLOCKIFY_API_KEY", "secret")
        .env("CFD_WORKSPACE", "w1")
        .env("CFD_BASE_URL", server.base_url())
        .output()
        .unwrap();
    assert!(output.status.success());
    assert_eq!(stdout(&output), "e1\n");

    let requests = server.requests();
    assert_eq!(
        requests[2].body,
        "{\"description\":\"Focus\",\"end\":\"2026-04-23T09:40:00+00:00\",\"start\":\"2026-04-23T09:07:00+00:00\"}"
    );
}

#[test]
fn entry_update_excludes_its_own_id_from_overlap_check() {
    let server = TestServer::spawn(vec![
        MockResponse::ok(r#"{"id":"u1","name":"Ada","email":"ada@example.com"}"#),
        MockResponse::ok(
            r#"[{"id":"e1","workspaceId":"w1","userId":"u1","description":"Focus","timeInterval":{"start":"2026-04-23T09:00:00Z","end":"2026-04-23T10:00:00Z","duration":"PT1H"}}]"#,
        ),
        MockResponse::ok(
            r#"{"id":"e1","workspaceId":"w1","userId":"u1","description":"Focus updated","timeInterval":{"start":"2026-04-23T09:15:00Z","end":"2026-04-23T10:15:00Z","duration":"PT1H"}}"#,
        ),
    ]);

    let output = bin()
        .args([
            "entry",
            "update",
            "e1",
            "--start",
            "2026-04-23T09:15:00Z",
            "--end",
            "2026-04-23T10:15:00Z",
            "--description",
            "Focus updated",
        ])
        .env("CLOCKIFY_API_KEY", "secret")
        .env("CFD_WORKSPACE", "w1")
        .env("CFD_BASE_URL", server.base_url())
        .output()
        .unwrap();

    assert!(output.status.success());
    assert_eq!(stdout(&output), "e1\n");
    assert!(!stderr(&output).contains("warning: overlaps existing entries"));
    assert!(!stderr(&output).contains("Continue despite overlap?"));

    let requests = server.requests();
    assert_eq!(
        requests[1].path,
        "/api/v1/workspaces/w1/user/u1/time-entries"
    );
    assert_eq!(requests[2].path, "/api/v1/workspaces/w1/time-entries/e1");
}

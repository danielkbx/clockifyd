mod support;

use std::fs;

use support::{bin, stderr, stdout, MockResponse, TestServer};

#[test]
fn timer_current_supports_text_json_and_no_meta() {
    let body = r#"[{"id":"e1","workspaceId":"w1","userId":"u1","projectId":"p1","description":"Run","timeInterval":{"start":"2026-04-23T09:00:00Z"}}]"#;
    let server = TestServer::spawn(vec![
        MockResponse::ok(r#"{"id":"u1","name":"Ada","email":"ada@example.com"}"#),
        MockResponse::ok(body),
        MockResponse::ok(r#"{"id":"p1","name":"Project One","workspaceId":"w1"}"#),
        MockResponse::ok(r#"{"id":"u1","name":"Ada","email":"ada@example.com"}"#),
        MockResponse::ok(body),
        MockResponse::ok(r#"{"id":"u1","name":"Ada","email":"ada@example.com"}"#),
        MockResponse::ok(body),
        MockResponse::ok(r#"{"id":"p1","name":"Project One","workspaceId":"w1"}"#),
    ]);

    let text = bin()
        .args(["timer", "current"])
        .env("CLOCKIFY_API_KEY", "secret")
        .env("CFD_WORKSPACE", "w1")
        .env("CFD_BASE_URL", server.base_url())
        .output()
        .unwrap();
    assert!(text.status.success());
    let text_stdout = stdout(&text);
    assert!(text_stdout.contains("id: e1\n"));
    assert!(text_stdout.contains("start: 2026-04-23T09:00:00Z\n"));
    assert!(text_stdout.contains("duration: "));
    assert!(text_stdout.contains("projectId: p1\n"));
    assert!(text_stdout.contains("project: Project One\n"));
    assert!(text_stdout.contains("\ndescription: Run\n"));

    let json = bin()
        .args(["timer", "current", "--format", "json"])
        .env("CLOCKIFY_API_KEY", "secret")
        .env("CFD_WORKSPACE", "w1")
        .env("CFD_BASE_URL", server.base_url())
        .output()
        .unwrap();
    assert!(json.status.success());
    assert!(stdout(&json).contains("\"description\": \"Run\""));
    assert!(stdout(&json).contains("\"projectId\": \"p1\""));

    let no_meta = bin()
        .args(["timer", "current", "--no-meta"])
        .env("CLOCKIFY_API_KEY", "secret")
        .env("CFD_WORKSPACE", "w1")
        .env("CFD_BASE_URL", server.base_url())
        .output()
        .unwrap();
    assert!(no_meta.status.success());
    let no_meta_stdout = stdout(&no_meta);
    assert!(no_meta_stdout.contains("start: 2026-04-23T09:00:00Z\n"));
    assert!(no_meta_stdout.contains("duration: "));
    assert!(no_meta_stdout.contains("projectId: p1\n"));
    assert!(no_meta_stdout.contains("project: Project One\n"));
    assert!(no_meta_stdout.contains("\ndescription: Run\n"));
    assert!(!no_meta_stdout.contains("id: "));
}

#[test]
fn timer_start_reports_running_timer() {
    let server = TestServer::spawn(vec![
        MockResponse::ok(r#"{"id":"u1","name":"Ada","email":"ada@example.com"}"#),
        MockResponse::ok(
            r#"[{"id":"e1","workspaceId":"w1","userId":"u1","description":"Run","timeInterval":{"start":"2026-04-23T09:00:00Z"}}]"#,
        ),
    ]);

    let output = bin()
        .args(["timer", "start"])
        .env("CLOCKIFY_API_KEY", "secret")
        .env("CFD_WORKSPACE", "w1")
        .env("CFD_BASE_URL", server.base_url())
        .output()
        .unwrap();

    assert!(!output.status.success());
    assert!(stderr(&output).contains("timer already running"));
}

#[test]
fn timer_stop_requires_running_timer() {
    let server = TestServer::spawn(vec![
        MockResponse::ok(r#"{"id":"u1","name":"Ada","email":"ada@example.com"}"#),
        MockResponse::ok("[]"),
    ]);

    let output = bin()
        .args(["timer", "stop"])
        .env("CLOCKIFY_API_KEY", "secret")
        .env("CFD_WORKSPACE", "w1")
        .env("CFD_BASE_URL", server.base_url())
        .output()
        .unwrap();

    assert!(!output.status.success());
    assert!(stderr(&output).contains("no running timer"));
}

#[test]
fn timer_stop_supports_text_json_and_no_meta() {
    let running =
        r#"[{"id":"e1","workspaceId":"w1","userId":"u1","projectId":"p1","description":"Run","timeInterval":{"start":"2026-04-23T09:00:00Z"}}]"#;
    let stopped =
        r#"{"id":"e1","workspaceId":"w1","userId":"u1","projectId":"p1","description":"Run","timeInterval":{"start":"2026-04-23T09:00:00Z","end":"2026-04-23T10:02:03Z"}}"#;
    let project = r#"{"id":"p1","name":"Project One","workspaceId":"w1"}"#;
    let server = TestServer::spawn(vec![
        MockResponse::ok(r#"{"id":"u1","name":"Ada","email":"ada@example.com"}"#),
        MockResponse::ok(running),
        MockResponse::ok("[]"),
        MockResponse::ok(stopped),
        MockResponse::ok(project),
        MockResponse::ok(r#"{"id":"u1","name":"Ada","email":"ada@example.com"}"#),
        MockResponse::ok(running),
        MockResponse::ok("[]"),
        MockResponse::ok(stopped),
        MockResponse::ok(r#"{"id":"u1","name":"Ada","email":"ada@example.com"}"#),
        MockResponse::ok(running),
        MockResponse::ok("[]"),
        MockResponse::ok(stopped),
        MockResponse::ok(project),
    ]);

    let text = bin()
        .args(["timer", "stop", "--end", "2026-04-23T10:02:03Z"])
        .env("CLOCKIFY_API_KEY", "secret")
        .env("CFD_WORKSPACE", "w1")
        .env("CFD_BASE_URL", server.base_url())
        .output()
        .unwrap();
    assert!(text.status.success());
    let text_stdout = stdout(&text);
    assert!(text_stdout.contains("id: e1\n"));
    assert!(text_stdout.contains("start: 2026-04-23T09:00:00Z\n"));
    assert!(text_stdout.contains("duration: 1h2m3s\n"));
    assert!(text_stdout.contains("projectId: p1\n"));
    assert!(text_stdout.contains("project: Project One\n"));
    assert!(text_stdout.contains("\ndescription: Run\n"));

    let json = bin()
        .args(["timer", "stop", "--end", "2026-04-23T10:02:03Z", "--format", "json"])
        .env("CLOCKIFY_API_KEY", "secret")
        .env("CFD_WORKSPACE", "w1")
        .env("CFD_BASE_URL", server.base_url())
        .output()
        .unwrap();
    assert!(json.status.success());
    assert!(stdout(&json).contains("\"projectId\": \"p1\""));
    assert!(stdout(&json).contains("\"end\": \"2026-04-23T10:02:03Z\""));

    let no_meta = bin()
        .args(["timer", "stop", "--end", "2026-04-23T10:02:03Z", "--no-meta"])
        .env("CLOCKIFY_API_KEY", "secret")
        .env("CFD_WORKSPACE", "w1")
        .env("CFD_BASE_URL", server.base_url())
        .output()
        .unwrap();
    assert!(no_meta.status.success());
    let no_meta_stdout = stdout(&no_meta);
    assert!(no_meta_stdout.contains("start: 2026-04-23T09:00:00Z\n"));
    assert!(no_meta_stdout.contains("duration: 1h2m3s\n"));
    assert!(no_meta_stdout.contains("projectId: p1\n"));
    assert!(no_meta_stdout.contains("project: Project One\n"));
    assert!(no_meta_stdout.contains("\ndescription: Run\n"));
    assert!(!no_meta_stdout.contains("id: "));
}

#[test]
fn timer_start_no_rounding_overrides_config_and_yes_skips_prompt() {
    let (_dir, config_path) = support::temp_config_path();
    fs::write(
        &config_path,
        "{\n  \"project\": \"p1\",\n  \"rounding\": \"15m\"\n}\n",
    )
    .unwrap();
    let server = TestServer::spawn(vec![
        MockResponse::ok(r#"{"id":"u1","name":"Ada","email":"ada@example.com"}"#),
        MockResponse::ok("[]"),
        MockResponse::ok(
            r#"[{"id":"e2","workspaceId":"w1","userId":"u1","description":"Overlap","timeInterval":{"start":"2026-04-23T09:00:00Z","end":"2026-04-23T10:00:00Z","duration":"PT1H"}}]"#,
        ),
        MockResponse::ok(
            r#"{"id":"e1","workspaceId":"w1","userId":"u1","description":"Run","timeInterval":{"start":"2026-04-23T09:07:00+00:00"}}"#,
        ),
    ]);

    let output = bin()
        .args([
            "timer",
            "start",
            "--start",
            "2026-04-23T09:07:00Z",
            "--description",
            "Run",
            "--no-rounding",
            "-y",
        ])
        .env("CFD_CONFIG", &config_path)
        .env("CLOCKIFY_API_KEY", "secret")
        .env("CFD_WORKSPACE", "w1")
        .env("CFD_BASE_URL", server.base_url())
        .output()
        .unwrap();
    assert!(output.status.success());
    assert_eq!(stdout(&output), "e1\n");
    assert!(stderr(&output).contains("warning: overlaps existing entries: e2"));
    assert!(!stderr(&output).contains("Continue despite overlap?"));

    let requests = server.requests();
    assert_eq!(
        requests[3].body,
        "{\"description\":\"Run\",\"projectId\":\"p1\",\"start\":\"2026-04-23T09:07:00+00:00\"}"
    );
}

#[test]
fn timer_start_requires_project_context() {
    let (_dir, config_path) = support::temp_config_path();
    fs::write(&config_path, "{\n  \"rounding\": \"15m\"\n}\n").unwrap();
    let server = TestServer::spawn(vec![
        MockResponse::ok(r#"{"id":"u1","name":"Ada","email":"ada@example.com"}"#),
        MockResponse::ok("[]"),
    ]);

    let output = bin()
        .args(["timer", "start", "--description", "Run"])
        .env("CFD_CONFIG", &config_path)
        .env("CLOCKIFY_API_KEY", "secret")
        .env("CFD_WORKSPACE", "w1")
        .env("CFD_BASE_URL", server.base_url())
        .output()
        .unwrap();

    assert!(!output.status.success());
    assert!(stderr(&output).contains("missing project"));
    assert!(stderr(&output).contains("cfd config set project <id>"));
}

#[test]
fn timer_start_flag_project_overrides_config() {
    let (_dir, config_path) = support::temp_config_path();
    fs::write(&config_path, "{\n  \"project\": \"stored-project\"\n}\n").unwrap();
    let server = TestServer::spawn(vec![
        MockResponse::ok(r#"{"id":"u1","name":"Ada","email":"ada@example.com"}"#),
        MockResponse::ok("[]"),
        MockResponse::ok("[]"),
        MockResponse::ok(
            r#"{"id":"e1","workspaceId":"w1","userId":"u1","projectId":"flag-project","description":"Run","timeInterval":{"start":"2026-04-23T09:07:00+00:00"}}"#,
        ),
    ]);

    let output = bin()
        .args([
            "timer",
            "start",
            "--start",
            "2026-04-23T09:07:00Z",
            "--project",
            "flag-project",
            "--description",
            "Run",
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
        requests[3].body,
        "{\"description\":\"Run\",\"projectId\":\"flag-project\",\"start\":\"2026-04-23T09:07:00+00:00\"}"
    );
}

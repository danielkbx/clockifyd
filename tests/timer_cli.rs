mod support;

use std::fs;
use std::process::Stdio;

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
        MockResponse::ok(r#"{"id":"p1","name":"Project One","workspaceId":"w1"}"#),
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
    let (_dir, config_path) = support::temp_config_path();
    fs::write(&config_path, "{\n  \"project\": \"p1\"\n}\n").unwrap();
    let server = TestServer::spawn(vec![
        MockResponse::ok(r#"{"id":"u1","name":"Ada","email":"ada@example.com"}"#),
        MockResponse::ok(
            r#"[{"id":"e1","workspaceId":"w1","userId":"u1","description":"Run","timeInterval":{"start":"2026-04-23T09:00:00Z"}}]"#,
        ),
    ]);

    let output = bin()
        .args(["timer", "start"])
        .env("CFD_CONFIG", &config_path)
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
    let running = r#"[{"id":"e1","workspaceId":"w1","userId":"u1","projectId":"p1","description":"Run","timeInterval":{"start":"2026-04-23T09:00:00Z"}}]"#;
    let stopped = r#"{"id":"e1","workspaceId":"w1","userId":"u1","projectId":"p1","description":"Run","timeInterval":{"start":"2026-04-23T09:00:00Z","end":"2026-04-23T10:02:03Z"}}"#;
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
        MockResponse::ok(project),
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
        .args([
            "timer",
            "stop",
            "--end",
            "2026-04-23T10:02:03Z",
            "--format",
            "json",
        ])
        .env("CLOCKIFY_API_KEY", "secret")
        .env("CFD_WORKSPACE", "w1")
        .env("CFD_BASE_URL", server.base_url())
        .output()
        .unwrap();
    assert!(json.status.success());
    assert!(stdout(&json).contains("\"projectId\": \"p1\""));
    assert!(stdout(&json).contains("\"end\": \"2026-04-23T10:02:03Z\""));

    let no_meta = bin()
        .args([
            "timer",
            "stop",
            "--end",
            "2026-04-23T10:02:03Z",
            "--no-meta",
        ])
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
            "Run",
            "--start",
            "2026-04-23T09:07:00Z",
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
        .args(["timer", "start", "Run"])
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
            "Run",
            "--start",
            "2026-04-23T09:07:00Z",
            "--project",
            "flag-project",
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

#[test]
fn timer_start_accepts_relative_start() {
    let server = TestServer::spawn(vec![
        MockResponse::ok(r#"{"id":"u1","name":"Ada","email":"ada@example.com"}"#),
        MockResponse::ok("[]"),
        MockResponse::ok("[]"),
        MockResponse::ok(
            r#"{"id":"e1","workspaceId":"w1","userId":"u1","projectId":"p1","description":"Run","timeInterval":{"start":"2026-04-23T09:50:00+00:00"}}"#,
        ),
    ]);

    let output = bin()
        .args([
            "timer",
            "start",
            "Run",
            "--project",
            "p1",
            "--start",
            "-10m",
            "--no-rounding",
            "-y",
        ])
        .env("CLOCKIFY_API_KEY", "secret")
        .env("CFD_WORKSPACE", "w1")
        .env("CFD_BASE_URL", server.base_url())
        .output()
        .unwrap();
    assert!(output.status.success());
    assert_eq!(stdout(&output), "e1\n");

    let requests = server.requests();
    let body: serde_json::Value = serde_json::from_str(&requests[3].body).unwrap();
    assert_eq!(body["description"], "Run");
    assert_eq!(body["projectId"], "p1");
    assert_ne!(body["start"], "-10m");
    assert!(body["start"].as_str().unwrap().starts_with("20"));
}

#[test]
fn timer_start_rejects_description_flag() {
    let (_dir, config_path) = support::temp_config_path();
    fs::write(&config_path, "{\n  \"project\": \"p1\"\n}\n").unwrap();
    let server = TestServer::spawn(vec![]);

    let output = bin()
        .args(["timer", "start", "--description", "Run"])
        .env("CFD_CONFIG", &config_path)
        .env("CLOCKIFY_API_KEY", "secret")
        .env("CFD_WORKSPACE", "w1")
        .env("CFD_BASE_URL", server.base_url())
        .output()
        .unwrap();

    assert!(!output.status.success());
    assert!(stderr(&output).contains("usage: cfd timer start [description]"));
    assert!(server.requests().is_empty());
}

#[test]
fn timer_start_rejects_multiple_positional_description_tokens() {
    let (_dir, config_path) = support::temp_config_path();
    fs::write(&config_path, "{\n  \"project\": \"p1\"\n}\n").unwrap();
    let server = TestServer::spawn(vec![]);

    let output = bin()
        .args(["timer", "start", "Run", "extra"])
        .env("CFD_CONFIG", &config_path)
        .env("CLOCKIFY_API_KEY", "secret")
        .env("CFD_WORKSPACE", "w1")
        .env("CFD_BASE_URL", server.base_url())
        .output()
        .unwrap();

    assert!(!output.status.success());
    assert!(stderr(&output).contains("usage: cfd timer start [description]"));
    assert!(server.requests().is_empty());
}

#[test]
fn timer_resume_direct_newest_copies_entry_fields() {
    let recent_entries = r#"[
        {"id":"older","workspaceId":"w1","userId":"u1","projectId":"p-old","description":"[CFD-TEST] resume older","timeInterval":{"start":"2026-04-23T09:00:00Z","end":"2026-04-23T09:30:00Z","duration":"PT30M"}},
        {"id":"newest","workspaceId":"w1","userId":"u1","projectId":"p-new","taskId":"t1","tagIds":["tag1"],"description":"[CFD-TEST] resume newest","timeInterval":{"start":"2026-04-23T10:00:00Z","end":"2026-04-23T10:30:00Z","duration":"PT30M"}}
    ]"#;
    let server = TestServer::spawn(vec![
        MockResponse::ok(r#"{"id":"u1","name":"Ada","email":"ada@example.com"}"#),
        MockResponse::ok("[]"),
        MockResponse::ok(recent_entries),
        MockResponse::ok(r#"{"id":"p-new","name":"Project New","workspaceId":"w1"}"#),
        MockResponse::ok(r#"{"id":"u1","name":"Ada","email":"ada@example.com"}"#),
        MockResponse::ok("[]"),
        MockResponse::ok("[]"),
        MockResponse::ok(
            r#"{"id":"resumed","workspaceId":"w1","userId":"u1","projectId":"p-new","taskId":"t1","tagIds":["tag1"],"description":"[CFD-TEST] resume newest","timeInterval":{"start":"2026-04-23T11:00:00+00:00"}}"#,
        ),
    ]);

    let output = bin()
        .args([
            "timer",
            "resume",
            "-1",
            "--start",
            "2026-04-23T11:00:00Z",
            "--no-rounding",
            "-y",
        ])
        .env("CLOCKIFY_API_KEY", "secret")
        .env("CFD_WORKSPACE", "w1")
        .env("CFD_BASE_URL", server.base_url())
        .output()
        .unwrap();

    assert!(output.status.success());
    assert_eq!(stdout(&output), "resumed\n");
    assert!(stderr(&output).contains("Selected entry:"));
    assert!(stderr(&output).contains("2026-04-23"));
    assert!(stderr(&output).contains("[CFD-TEST] resume newest"));
    assert!(!stderr(&output).contains("Resume this entry?"));

    let requests = server.requests();
    let body: serde_json::Value = serde_json::from_str(&requests[7].body).unwrap();
    assert_eq!(body["projectId"], "p-new");
    assert_eq!(body["taskId"], "t1");
    assert_eq!(body["tagIds"], serde_json::json!(["tag1"]));
    assert_eq!(body["description"], "[CFD-TEST] resume newest");
    assert_eq!(body["start"], "2026-04-23T11:00:00+00:00");
}

#[test]
fn timer_resume_direct_second_newest_uses_minus_two() {
    let recent_entries = r#"[
        {"id":"older","workspaceId":"w1","userId":"u1","projectId":"p-old","description":"[CFD-TEST] resume older","timeInterval":{"start":"2026-04-23T09:00:00Z","end":"2026-04-23T09:30:00Z","duration":"PT30M"}},
        {"id":"newest","workspaceId":"w1","userId":"u1","projectId":"p-new","description":"[CFD-TEST] resume newest","timeInterval":{"start":"2026-04-23T10:00:00Z","end":"2026-04-23T10:30:00Z","duration":"PT30M"}}
    ]"#;
    let server = TestServer::spawn(vec![
        MockResponse::ok(r#"{"id":"u1","name":"Ada","email":"ada@example.com"}"#),
        MockResponse::ok("[]"),
        MockResponse::ok(recent_entries),
        MockResponse::ok(r#"{"id":"p-old","name":"Project Old","workspaceId":"w1"}"#),
        MockResponse::ok(r#"{"id":"u1","name":"Ada","email":"ada@example.com"}"#),
        MockResponse::ok("[]"),
        MockResponse::ok("[]"),
        MockResponse::ok(
            r#"{"id":"resumed","workspaceId":"w1","userId":"u1","projectId":"p-old","description":"[CFD-TEST] resume older","timeInterval":{"start":"2026-04-23T11:00:00+00:00"}}"#,
        ),
    ]);

    let output = bin()
        .args([
            "timer",
            "resume",
            "-2",
            "--start",
            "2026-04-23T11:00:00Z",
            "--no-rounding",
            "-y",
        ])
        .env("CLOCKIFY_API_KEY", "secret")
        .env("CFD_WORKSPACE", "w1")
        .env("CFD_BASE_URL", server.base_url())
        .output()
        .unwrap();

    assert!(output.status.success());
    assert_eq!(stdout(&output), "resumed\n");
    assert!(stderr(&output).contains("[CFD-TEST] resume older"));

    let requests = server.requests();
    let body: serde_json::Value = serde_json::from_str(&requests[7].body).unwrap();
    assert_eq!(body["projectId"], "p-old");
    assert_eq!(body["description"], "[CFD-TEST] resume older");
}

#[test]
fn timer_resume_requires_interactive_terminal_without_yes() {
    let server = TestServer::spawn(vec![]);

    let output = bin()
        .args(["timer", "resume", "-1"])
        .env("CLOCKIFY_API_KEY", "secret")
        .env("CFD_WORKSPACE", "w1")
        .env("CFD_BASE_URL", server.base_url())
        .stdin(Stdio::null())
        .output()
        .unwrap();

    assert!(!output.status.success());
    assert!(stderr(&output).contains("cfd timer resume requires an interactive terminal"));
    assert!(server.requests().is_empty());
}

#[test]
fn timer_resume_reports_no_recent_entries() {
    let server = TestServer::spawn(vec![
        MockResponse::ok(r#"{"id":"u1","name":"Ada","email":"ada@example.com"}"#),
        MockResponse::ok("[]"),
        MockResponse::ok("[]"),
    ]);

    let output = bin()
        .args(["timer", "resume", "-1", "-y"])
        .env("CLOCKIFY_API_KEY", "secret")
        .env("CFD_WORKSPACE", "w1")
        .env("CFD_BASE_URL", server.base_url())
        .output()
        .unwrap();

    assert!(!output.status.success());
    assert!(stderr(&output).contains("no recent entries to resume"));
}

#[test]
fn timer_resume_reports_missing_direct_selector_entry() {
    let recent_entries = r#"[{"id":"newest","workspaceId":"w1","userId":"u1","projectId":"p1","description":"Run","timeInterval":{"start":"2026-04-23T10:00:00Z","end":"2026-04-23T10:30:00Z","duration":"PT30M"}}]"#;
    let server = TestServer::spawn(vec![
        MockResponse::ok(r#"{"id":"u1","name":"Ada","email":"ada@example.com"}"#),
        MockResponse::ok("[]"),
        MockResponse::ok(recent_entries),
    ]);

    let output = bin()
        .args(["timer", "resume", "-9", "-y"])
        .env("CLOCKIFY_API_KEY", "secret")
        .env("CFD_WORKSPACE", "w1")
        .env("CFD_BASE_URL", server.base_url())
        .output()
        .unwrap();

    assert!(!output.status.success());
    assert!(stderr(&output).contains("recent entry not found: -9"));
}

#[test]
fn timer_resume_reports_running_timer() {
    let running = r#"[{"id":"running","workspaceId":"w1","userId":"u1","projectId":"p1","description":"Call","timeInterval":{"start":"2026-04-23T10:00:00Z"}}]"#;
    let server = TestServer::spawn(vec![
        MockResponse::ok(r#"{"id":"u1","name":"Ada","email":"ada@example.com"}"#),
        MockResponse::ok(running),
    ]);

    let output = bin()
        .args(["timer", "resume", "-1", "-y"])
        .env("CLOCKIFY_API_KEY", "secret")
        .env("CFD_WORKSPACE", "w1")
        .env("CFD_BASE_URL", server.base_url())
        .output()
        .unwrap();

    assert!(!output.status.success());
    assert!(stderr(&output).contains("timer already running"));
}

#[test]
fn timer_resume_rejects_multiple_numeric_selectors_and_field_overrides() {
    let server = TestServer::spawn(vec![]);

    let multiple = bin()
        .args(["timer", "resume", "-1", "-2", "-y"])
        .env("CLOCKIFY_API_KEY", "secret")
        .env("CFD_WORKSPACE", "w1")
        .env("CFD_BASE_URL", server.base_url())
        .output()
        .unwrap();
    assert!(!multiple.status.success());
    assert!(stderr(&multiple).contains("use only one resume selector"));

    let project = bin()
        .args(["timer", "resume", "-1", "--project", "p1", "-y"])
        .env("CLOCKIFY_API_KEY", "secret")
        .env("CFD_WORKSPACE", "w1")
        .env("CFD_BASE_URL", server.base_url())
        .output()
        .unwrap();
    assert!(!project.status.success());
    assert!(stderr(&project).contains("does not accept --project"));

    assert!(server.requests().is_empty());
}

#[test]
fn timer_resume_rejects_interactive_options_with_direct_selectors() {
    let server = TestServer::spawn(vec![]);

    let filter = bin()
        .args(["timer", "resume", "-1", "needle", "-y"])
        .env("CLOCKIFY_API_KEY", "secret")
        .env("CFD_WORKSPACE", "w1")
        .env("CFD_BASE_URL", server.base_url())
        .output()
        .unwrap();
    assert!(!filter.status.success());
    assert!(stderr(&filter).contains("filters are only supported for interactive timer resume"));

    let limit = bin()
        .args(["timer", "resume", "-1", "-n2", "-y"])
        .env("CLOCKIFY_API_KEY", "secret")
        .env("CFD_WORKSPACE", "w1")
        .env("CFD_BASE_URL", server.base_url())
        .output()
        .unwrap();
    assert!(!limit.status.success());
    assert!(stderr(&limit).contains("-n<count> is only supported for interactive timer resume"));

    assert!(server.requests().is_empty());
}

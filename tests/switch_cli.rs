mod support;

use std::fs;

use serde_json::Value;
use support::{bin, stderr, stdout, MockResponse, TestServer};

#[test]
fn switch_current_without_active_switch_returns_inactive() {
    let (_dir, config_path) = support::temp_config_path();
    fs::write(&config_path, "{}\n").unwrap();

    let text = bin()
        .args(["switch", "current"])
        .env("CFD_CONFIG", &config_path)
        .env("CLOCKIFY_API_KEY", "secret")
        .env("CFD_WORKSPACE", "w1")
        .output()
        .unwrap();
    assert!(text.status.success(), "{}", stderr(&text));
    assert_eq!(stdout(&text), "active: no\n");

    let json = bin()
        .args(["switch", "current", "--format", "json"])
        .env("CFD_CONFIG", &config_path)
        .env("CLOCKIFY_API_KEY", "secret")
        .env("CFD_WORKSPACE", "w1")
        .output()
        .unwrap();
    assert!(json.status.success(), "{}", stderr(&json));
    let value: Value = serde_json::from_str(&stdout(&json)).unwrap();
    assert_eq!(value["active"], false);
    assert!(value["current"].is_null());
    assert!(value["returnsTo"].is_null());
}

#[test]
fn switch_current_json_includes_current_timer_and_return_target() {
    let (_dir, config_path) = support::temp_config_path();
    fs::write(&config_path, active_switch_config()).unwrap();
    let server = TestServer::spawn(vec![
        MockResponse::ok(r#"{"id":"u1","name":"Ada","email":"ada@example.com"}"#),
        MockResponse::ok(
            r#"[{"id":"b1","workspaceId":"w1","userId":"u1","projectId":"p2","taskId":"t2","tagIds":["tag2"],"description":"Support call","timeInterval":{"start":"2026-05-06T10:15:00Z"}}]"#,
        ),
        MockResponse::ok(r#"{"id":"p2","name":"Support","workspaceId":"w1"}"#),
        MockResponse::ok(r#"{"id":"p1","name":"Main project","workspaceId":"w1"}"#),
    ]);

    let output = bin()
        .args(["switch", "current", "--format", "json"])
        .env("CFD_CONFIG", &config_path)
        .env("CLOCKIFY_API_KEY", "secret")
        .env("CFD_WORKSPACE", "w1")
        .env("CFD_BASE_URL", server.base_url())
        .output()
        .unwrap();

    assert!(output.status.success(), "{}", stderr(&output));
    let value: Value = serde_json::from_str(&stdout(&output)).unwrap();
    assert_eq!(value["active"], true);
    assert_eq!(value["current"]["id"], "b1");
    assert_eq!(value["current"]["projectId"], "p2");
    assert_eq!(value["current"]["projectName"], "Support");
    assert_eq!(value["current"]["taskId"], "t2");
    assert_eq!(value["current"]["tagIds"], serde_json::json!(["tag2"]));
    assert_eq!(value["current"]["description"], "Support call");
    assert_eq!(value["current"]["start"], "2026-05-06T10:15:00Z");
    assert!(value["current"]["durationSeconds"].is_number());
    assert!(value["current"]["duration"].is_string());
    assert_eq!(value["returnsTo"]["originalEntryId"], "a1");
    assert_eq!(value["returnsTo"]["switchedAt"], "2026-05-06T10:15:00Z");
    assert_eq!(value["returnsTo"]["projectId"], "p1");
    assert_eq!(value["returnsTo"]["projectName"], "Main project");
    assert_eq!(value["returnsTo"]["taskId"], "t1");
    assert_eq!(value["returnsTo"]["tagIds"], serde_json::json!(["tag1"]));
    assert_eq!(value["returnsTo"]["description"], "Timer A");
}

#[test]
fn switch_current_text_includes_current_and_returns_to_sections() {
    let (_dir, config_path) = support::temp_config_path();
    fs::write(&config_path, active_switch_config()).unwrap();
    let responses = vec![
        MockResponse::ok(r#"{"id":"u1","name":"Ada","email":"ada@example.com"}"#),
        MockResponse::ok(
            r#"[{"id":"b1","workspaceId":"w1","userId":"u1","projectId":"p2","taskId":"t2","description":"Support call","timeInterval":{"start":"2026-05-06T10:15:00Z"}}]"#,
        ),
        MockResponse::ok(r#"{"id":"p2","name":"Support","workspaceId":"w1"}"#),
        MockResponse::ok(r#"{"id":"p1","name":"Main project","workspaceId":"w1"}"#),
        MockResponse::ok(r#"{"id":"u1","name":"Ada","email":"ada@example.com"}"#),
        MockResponse::ok(
            r#"[{"id":"b1","workspaceId":"w1","userId":"u1","projectId":"p2","taskId":"t2","description":"Support call","timeInterval":{"start":"2026-05-06T10:15:00Z"}}]"#,
        ),
        MockResponse::ok(r#"{"id":"p2","name":"Support","workspaceId":"w1"}"#),
        MockResponse::ok(r#"{"id":"p1","name":"Main project","workspaceId":"w1"}"#),
    ];
    let server = TestServer::spawn(responses);

    let text = bin()
        .args(["switch", "current"])
        .env("CFD_CONFIG", &config_path)
        .env("CLOCKIFY_API_KEY", "secret")
        .env("CFD_WORKSPACE", "w1")
        .env("CFD_BASE_URL", server.base_url())
        .output()
        .unwrap();
    assert!(text.status.success(), "{}", stderr(&text));
    let text_stdout = stdout(&text);
    assert!(text_stdout.contains("active: yes\n\ncurrent:\n"));
    assert!(text_stdout.contains("id: b1\n"));
    assert!(text_stdout.contains("projectId: p2\n"));
    assert!(text_stdout.contains("project: Support\n"));
    assert!(text_stdout.contains("taskId: t2\n"));
    assert!(text_stdout.contains("description: Support call\n"));
    assert!(text_stdout.contains("\nreturnsTo:\n"));
    assert!(text_stdout.contains("originalEntryId: a1\n"));
    assert!(text_stdout.contains("switchedAt: 2026-05-06T10:15:00Z\n"));
    assert!(text_stdout.contains("projectId: p1\n"));
    assert!(text_stdout.contains("project: Main project\n"));
    assert!(text_stdout.contains("description: Timer A\n"));

    let no_meta = bin()
        .args(["switch", "current", "--no-meta"])
        .env("CFD_CONFIG", &config_path)
        .env("CLOCKIFY_API_KEY", "secret")
        .env("CFD_WORKSPACE", "w1")
        .env("CFD_BASE_URL", server.base_url())
        .output()
        .unwrap();
    assert!(no_meta.status.success(), "{}", stderr(&no_meta));
    let no_meta_stdout = stdout(&no_meta);
    assert!(no_meta_stdout.contains("current:\n"));
    assert!(no_meta_stdout.contains("returnsTo:\n"));
    assert!(no_meta_stdout.contains("project: Support\n"));
    assert!(no_meta_stdout.contains("project: Main project\n"));
    assert!(no_meta_stdout.contains("description: Support call\n"));
    assert!(no_meta_stdout.contains("description: Timer A\n"));
    assert!(!no_meta_stdout.contains("id: b1\n"));
    assert!(!no_meta_stdout.contains("originalEntryId: a1\n"));
    assert!(!no_meta_stdout.contains("projectId: p1\n"));
    assert!(!no_meta_stdout.contains("projectId: p2\n"));
}

#[test]
fn switch_current_stale_state_fails_without_mutating_config() {
    let (_dir, config_path) = support::temp_config_path();
    let config = active_switch_config();
    fs::write(&config_path, config).unwrap();
    let server = TestServer::spawn(vec![
        MockResponse::ok(r#"{"id":"u1","name":"Ada","email":"ada@example.com"}"#),
        MockResponse::ok(
            r#"[{"id":"other","workspaceId":"w1","userId":"u1","projectId":"p2","description":"Other","timeInterval":{"start":"2026-05-06T10:15:00Z"}}]"#,
        ),
    ]);

    let output = bin()
        .args(["switch", "current"])
        .env("CFD_CONFIG", &config_path)
        .env("CLOCKIFY_API_KEY", "secret")
        .env("CFD_WORKSPACE", "w1")
        .env("CFD_BASE_URL", server.base_url())
        .output()
        .unwrap();

    assert!(!output.status.success());
    assert!(stderr(&output)
        .contains("switch state is stale: current timer does not match switched timer"));
    assert_eq!(fs::read_to_string(&config_path).unwrap(), config);
    assert_eq!(server.requests().len(), 2);
}

#[test]
fn switch_start_and_stop_create_contiguous_entries_and_update_state() {
    let (_dir, config_path) = support::temp_config_path();
    fs::write(&config_path, "{}\n").unwrap();
    let server = TestServer::spawn(vec![
        MockResponse::ok(r#"{"id":"u1","name":"Ada","email":"ada@example.com"}"#),
        MockResponse::ok(
            r#"[{"id":"a-running","workspaceId":"w1","userId":"u1","projectId":"p1","taskId":"t1","tagIds":["tag1"],"description":"Timer A","timeInterval":{"start":"2026-05-06T09:00:00Z"}}]"#,
        ),
        MockResponse::ok(
            r#"{"id":"a1","workspaceId":"w1","userId":"u1","projectId":"p1","taskId":"t1","tagIds":["tag1"],"description":"Timer A","timeInterval":{"start":"2026-05-06T09:00:00Z","end":"2026-05-06T10:15:00+00:00"}}"#,
        ),
        MockResponse::ok(
            r#"{"id":"b1","workspaceId":"w1","userId":"u1","projectId":"p2","description":"Timer B","timeInterval":{"start":"2026-05-06T10:15:00+00:00"}}"#,
        ),
        MockResponse::ok(r#"{"id":"u1","name":"Ada","email":"ada@example.com"}"#),
        MockResponse::ok(
            r#"[{"id":"b1","workspaceId":"w1","userId":"u1","projectId":"p2","description":"Timer B","timeInterval":{"start":"2026-05-06T10:15:00+00:00"}}]"#,
        ),
        MockResponse::ok(
            r#"{"id":"b1","workspaceId":"w1","userId":"u1","projectId":"p2","description":"Timer B","timeInterval":{"start":"2026-05-06T10:15:00+00:00","end":"2026-05-06T10:45:00+00:00"}}"#,
        ),
        MockResponse::ok(
            r#"{"id":"a2","workspaceId":"w1","userId":"u1","projectId":"p1","taskId":"t1","tagIds":["tag1"],"description":"Timer A","timeInterval":{"start":"2026-05-06T10:45:00+00:00"}}"#,
        ),
    ]);

    let start = bin()
        .args([
            "switch",
            "start",
            "Timer B",
            "--project",
            "p2",
            "--start",
            "2026-05-06T10:15:00Z",
            "--no-rounding",
        ])
        .env("CFD_CONFIG", &config_path)
        .env("CLOCKIFY_API_KEY", "secret")
        .env("CFD_WORKSPACE", "w1")
        .env("CFD_BASE_URL", server.base_url())
        .output()
        .unwrap();
    assert!(start.status.success(), "{}", stderr(&start));
    assert_eq!(stdout(&start), "b1\n");

    let config_after_start: Value =
        serde_json::from_str(&fs::read_to_string(&config_path).unwrap()).unwrap();
    assert_eq!(config_after_start["activeSwitch"]["originalEntryId"], "a1");
    assert_eq!(config_after_start["activeSwitch"]["switchedEntryId"], "b1");
    assert_eq!(
        config_after_start["activeSwitch"]["returnStart"],
        "2026-05-06T09:00:00Z"
    );
    assert_eq!(
        config_after_start["activeSwitch"]["returnTo"]["projectId"],
        "p1"
    );

    let stop = bin()
        .args([
            "switch",
            "stop",
            "--end",
            "2026-05-06T10:45:00Z",
            "--no-rounding",
        ])
        .env("CFD_CONFIG", &config_path)
        .env("CLOCKIFY_API_KEY", "secret")
        .env("CFD_WORKSPACE", "w1")
        .env("CFD_BASE_URL", server.base_url())
        .output()
        .unwrap();
    assert!(stop.status.success(), "{}", stderr(&stop));
    assert_eq!(stdout(&stop), "a2\n");

    let config_after_stop: Value =
        serde_json::from_str(&fs::read_to_string(&config_path).unwrap()).unwrap();
    assert!(config_after_stop.get("activeSwitch").is_none());

    let requests = server.requests();
    assert_eq!(requests[2].method, "PATCH");
    assert_eq!(
        serde_json::from_str::<Value>(&requests[2].body).unwrap()["end"],
        "2026-05-06T10:15:00+00:00"
    );
    assert_eq!(requests[3].method, "POST");
    let start_b: Value = serde_json::from_str(&requests[3].body).unwrap();
    assert_eq!(start_b["start"], "2026-05-06T10:15:00+00:00");
    assert_eq!(start_b["projectId"], "p2");
    assert_eq!(requests[6].method, "PATCH");
    assert_eq!(
        serde_json::from_str::<Value>(&requests[6].body).unwrap()["end"],
        "2026-05-06T10:45:00+00:00"
    );
    assert_eq!(requests[7].method, "POST");
    let restart_a: Value = serde_json::from_str(&requests[7].body).unwrap();
    assert_eq!(restart_a["start"], "2026-05-06T10:45:00+00:00");
    assert_eq!(restart_a["projectId"], "p1");
    assert_eq!(restart_a["taskId"], "t1");
    assert_eq!(restart_a["tagIds"], serde_json::json!(["tag1"]));
    assert_eq!(restart_a["description"], "Timer A");
}

#[test]
fn timer_switch_resume_direct_selector_switches_to_recent_entry_fields() {
    let (_dir, config_path) = support::temp_config_path();
    fs::write(&config_path, "{}\n").unwrap();
    let recent_entries = r#"[
        {"id":"older","workspaceId":"w1","userId":"u1","projectId":"p-old","description":"Older","timeInterval":{"start":"2026-05-06T08:00:00Z","end":"2026-05-06T08:30:00Z","duration":"PT30M"}},
        {"id":"newest","workspaceId":"w1","userId":"u1","projectId":"p2","taskId":"t2","tagIds":["tag2"],"description":"Recent B","timeInterval":{"start":"2026-05-06T09:00:00Z","end":"2026-05-06T09:30:00Z","duration":"PT30M"}}
    ]"#;
    let server = TestServer::spawn(vec![
        MockResponse::ok(r#"{"id":"u1","name":"Ada","email":"ada@example.com"}"#),
        MockResponse::ok(recent_entries),
        MockResponse::ok(r#"{"id":"p2","name":"Support","workspaceId":"w1"}"#),
        MockResponse::ok(r#"{"id":"u1","name":"Ada","email":"ada@example.com"}"#),
        MockResponse::ok(
            r#"[{"id":"a-running","workspaceId":"w1","userId":"u1","projectId":"p1","description":"Timer A","timeInterval":{"start":"2026-05-06T10:00:00Z"}}]"#,
        ),
        MockResponse::ok(
            r#"{"id":"a1","workspaceId":"w1","userId":"u1","projectId":"p1","description":"Timer A","timeInterval":{"start":"2026-05-06T10:00:00Z","end":"2026-05-06T10:15:00+00:00"}}"#,
        ),
        MockResponse::ok(
            r#"{"id":"b1","workspaceId":"w1","userId":"u1","projectId":"p2","taskId":"t2","tagIds":["tag2"],"description":"Recent B","timeInterval":{"start":"2026-05-06T10:15:00+00:00"}}"#,
        ),
    ]);

    let output = bin()
        .args([
            "timer",
            "switch",
            "resume",
            "-1",
            "--start",
            "2026-05-06T10:15:00Z",
            "--no-rounding",
            "-y",
        ])
        .env("CFD_CONFIG", &config_path)
        .env("CLOCKIFY_API_KEY", "secret")
        .env("CFD_WORKSPACE", "w1")
        .env("CFD_BASE_URL", server.base_url())
        .output()
        .unwrap();

    assert!(output.status.success(), "{}", stderr(&output));
    assert_eq!(stdout(&output), "b1\n");
    assert!(stderr(&output).contains("Selected entry:"));
    assert!(stderr(&output).contains("Recent B"));

    let config_after: Value =
        serde_json::from_str(&fs::read_to_string(&config_path).unwrap()).unwrap();
    assert_eq!(config_after["activeSwitch"]["originalEntryId"], "a1");
    assert_eq!(config_after["activeSwitch"]["switchedEntryId"], "b1");
    assert_eq!(config_after["activeSwitch"]["returnTo"]["projectId"], "p1");

    let requests = server.requests();
    assert_eq!(requests[6].method, "POST");
    let body: Value = serde_json::from_str(&requests[6].body).unwrap();
    assert_eq!(body["projectId"], "p2");
    assert_eq!(body["taskId"], "t2");
    assert_eq!(body["tagIds"], serde_json::json!(["tag2"]));
    assert_eq!(body["description"], "Recent B");
    assert_eq!(body["start"], "2026-05-06T10:15:00+00:00");
}

fn active_switch_config() -> &'static str {
    r#"{
  "activeSwitch": {
    "workspaceId": "w1",
    "userId": "u1",
    "originalEntryId": "a1",
    "switchedEntryId": "b1",
    "switchedStart": "2026-05-06T10:15:00Z",
    "returnStart": "2026-05-06T09:00:00Z",
    "returnTo": {
      "projectId": "p1",
      "taskId": "t1",
      "tagIds": ["tag1"],
      "description": "Timer A"
    }
  }
}
"#
}

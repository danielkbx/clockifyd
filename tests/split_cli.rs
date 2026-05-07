mod support;

use std::fs;

use serde_json::Value;
use support::{bin, stderr, stdout, MockResponse, TestServer};

const USER: &str = r#"{"id":"u1","name":"Ada","email":"ada@example.com"}"#;
const PROJECT: &str = r#"{"id":"p1","name":"Project One","workspaceId":"w1"}"#;
const ENTRY: &str = r#"{"id":"e1","workspaceId":"w1","userId":"u1","projectId":"p1","taskId":"t1","tagIds":["tag1"],"description":"Focus","timeInterval":{"start":"2026-04-23T09:00:00Z","end":"2026-04-23T11:00:00Z","duration":"PT2H"}}"#;
const UPDATED_ENTRY: &str = r#"{"id":"e1","workspaceId":"w1","userId":"u1","projectId":"p1","taskId":"t1","tagIds":["tag1"],"description":"Focus","timeInterval":{"start":"2026-04-23T09:00:00Z","end":"2026-04-23T10:00:00+00:00","duration":"PT1H"}}"#;
const CREATED_ENTRY: &str = r#"{"id":"e2","workspaceId":"w1","userId":"u1","projectId":"p1","taskId":"t1","tagIds":["tag1"],"description":"Focus","timeInterval":{"start":"2026-04-23T10:00:00+00:00","end":"2026-04-23T11:00:00Z","duration":"PT1H"}}"#;
const RUNNING: &str = r#"[{"id":"timer1","workspaceId":"w1","userId":"u1","projectId":"p1","taskId":"t1","tagIds":["tag1"],"description":"Focus","timeInterval":{"start":"2026-04-23T09:00:00Z"}}]"#;
const STOPPED_TIMER: &str = r#"{"id":"timer1","workspaceId":"w1","userId":"u1","projectId":"p1","taskId":"t1","tagIds":["tag1"],"description":"Focus","timeInterval":{"start":"2026-04-23T09:00:00Z","end":"2026-04-23T10:00:00+00:00","duration":"PT1H"}}"#;
const CREATED_TIMER: &str = r#"{"id":"timer2","workspaceId":"w1","userId":"u1","projectId":"p1","taskId":"t1","tagIds":["tag1"],"description":"Focus","timeInterval":{"start":"2026-04-23T10:00:00+00:00"}}"#;

#[test]
fn help_split_works() {
    for args in [["help", "split"], ["split", "help"]] {
        let output = bin().args(args).output().unwrap();
        assert!(output.status.success());
        let text = stdout(&output);
        assert!(text.contains("cfd split entry <id> --at <time>"));
        assert!(text.contains("Gap is added after rounding --at"));
    }
}

#[test]
fn split_entry_updates_original_and_creates_copy_with_text_output() {
    let (_dir, config_path) = support::temp_config_path();
    fs::write(&config_path, "{\n  \"rounding\": \"15m\"\n}\n").unwrap();
    let server = TestServer::spawn(vec![
        MockResponse::ok(USER),
        MockResponse::ok(ENTRY),
        MockResponse::ok("[]"),
        MockResponse::ok("[]"),
        MockResponse::ok(UPDATED_ENTRY),
        MockResponse::ok(CREATED_ENTRY),
        MockResponse::ok(PROJECT),
        MockResponse::ok(PROJECT),
    ]);

    let output = bin()
        .args([
            "split",
            "entry",
            "e1",
            "--at",
            "2026-04-23T10:07:00Z",
            "--gap",
            "5m",
        ])
        .env("CFD_CONFIG", &config_path)
        .env("CLOCKIFY_API_KEY", "secret")
        .env("CFD_WORKSPACE", "w1")
        .env("CFD_BASE_URL", server.base_url())
        .output()
        .unwrap();

    assert!(output.status.success(), "{}", stderr(&output));
    let text = stdout(&output);
    assert!(text.contains("updated:\nid: e1\n"));
    assert!(text.contains("created:\nid: e2\n"));
    assert!(text.contains("projectName: Project One\n"));
    assert!(text.contains("task: t1\n"));
    assert!(text.contains("tags: tag1\n"));

    let requests = server.requests();
    assert_eq!(requests[1].path, "/api/v1/workspaces/w1/time-entries/e1");
    assert_eq!(
        requests[2].path,
        "/api/v1/workspaces/w1/user/u1/time-entries"
    );
    assert_eq!(
        requests[3].path,
        "/api/v1/workspaces/w1/user/u1/time-entries"
    );
    assert_eq!(requests[4].method, "PUT");
    assert_eq!(requests[5].method, "POST");
    assert_eq!(
        requests[4].body,
        "{\"description\":\"Focus\",\"end\":\"2026-04-23T10:00:00+00:00\",\"projectId\":\"p1\",\"start\":\"2026-04-23T09:00:00Z\",\"tagIds\":[\"tag1\"],\"taskId\":\"t1\"}"
    );
    assert_eq!(
        requests[5].body,
        "{\"description\":\"Focus\",\"end\":\"2026-04-23T11:00:00Z\",\"projectId\":\"p1\",\"start\":\"2026-04-23T10:00:00+00:00\",\"tagIds\":[\"tag1\"],\"taskId\":\"t1\"}"
    );
}

#[test]
fn split_entry_json_and_no_rounding_preserve_unrounded_gap() {
    let (_dir, config_path) = support::temp_config_path();
    fs::write(&config_path, "{\n  \"rounding\": \"15m\"\n}\n").unwrap();
    let updated = r#"{"id":"e1","workspaceId":"w1","userId":"u1","projectId":"p1","description":"Focus","timeInterval":{"start":"2026-04-23T09:00:00Z","end":"2026-04-23T10:07:00+00:00"}}"#;
    let created = r#"{"id":"e2","workspaceId":"w1","userId":"u1","projectId":"p1","description":"Focus","timeInterval":{"start":"2026-04-23T10:12:00+00:00","end":"2026-04-23T11:00:00Z"}}"#;
    let server = TestServer::spawn(vec![
        MockResponse::ok(USER),
        MockResponse::ok(ENTRY),
        MockResponse::ok("[]"),
        MockResponse::ok("[]"),
        MockResponse::ok(updated),
        MockResponse::ok(created),
    ]);

    let output = bin()
        .args([
            "split",
            "entry",
            "e1",
            "--at",
            "2026-04-23T10:07:00Z",
            "--gap",
            "5m",
            "--no-rounding",
            "--format",
            "json",
        ])
        .env("CFD_CONFIG", &config_path)
        .env("CLOCKIFY_API_KEY", "secret")
        .env("CFD_WORKSPACE", "w1")
        .env("CFD_BASE_URL", server.base_url())
        .output()
        .unwrap();

    assert!(output.status.success(), "{}", stderr(&output));
    let value: Value = serde_json::from_str(&stdout(&output)).unwrap();
    assert_eq!(value["updated"]["id"], "e1");
    assert_eq!(value["created"]["id"], "e2");
    assert_eq!(
        value["created"]["timeInterval"]["start"],
        "2026-04-23T10:12:00+00:00"
    );

    let requests = server.requests();
    assert_eq!(
        requests[5].body,
        "{\"description\":\"Focus\",\"end\":\"2026-04-23T11:00:00Z\",\"projectId\":\"p1\",\"start\":\"2026-04-23T10:12:00+00:00\",\"tagIds\":[\"tag1\"],\"taskId\":\"t1\"}"
    );
}

#[test]
fn split_entry_no_meta_suppresses_ids_in_text_blocks() {
    let server = TestServer::spawn(vec![
        MockResponse::ok(USER),
        MockResponse::ok(ENTRY),
        MockResponse::ok("[]"),
        MockResponse::ok("[]"),
        MockResponse::ok(UPDATED_ENTRY),
        MockResponse::ok(CREATED_ENTRY),
        MockResponse::ok(PROJECT),
        MockResponse::ok(PROJECT),
    ]);

    let output = bin()
        .args([
            "split",
            "entry",
            "e1",
            "--at",
            "2026-04-23T10:00:00Z",
            "--no-meta",
        ])
        .env("CLOCKIFY_API_KEY", "secret")
        .env("CFD_WORKSPACE", "w1")
        .env("CFD_BASE_URL", server.base_url())
        .output()
        .unwrap();

    assert!(output.status.success(), "{}", stderr(&output));
    let text = stdout(&output);
    assert!(text.contains("updated:\nstart:"));
    assert!(text.contains("created:\nstart:"));
    assert!(!text.contains("id: e1"));
    assert!(!text.contains("id: e2"));
}

#[test]
fn split_entry_rejects_invalid_shapes_and_running_entries() {
    let missing_at = bin()
        .args(["split", "entry", "e1"])
        .env("CLOCKIFY_API_KEY", "secret")
        .env("CFD_WORKSPACE", "w1")
        .output()
        .unwrap();
    assert!(!missing_at.status.success());
    assert!(stderr(&missing_at).contains("usage: cfd split"));

    let extra = bin()
        .args([
            "split",
            "entry",
            "e1",
            "extra",
            "--at",
            "2026-04-23T10:00:00Z",
        ])
        .env("CLOCKIFY_API_KEY", "secret")
        .env("CFD_WORKSPACE", "w1")
        .output()
        .unwrap();
    assert!(!extra.status.success());
    assert!(stderr(&extra).contains("usage: cfd split entry <id>"));

    let server = TestServer::spawn(vec![
        MockResponse::ok(USER),
        MockResponse::ok(
            r#"{"id":"e1","workspaceId":"w1","userId":"u1","projectId":"p1","description":"Run","timeInterval":{"start":"2026-04-23T09:00:00Z"}}"#,
        ),
    ]);
    let running = bin()
        .args(["split", "entry", "e1", "--at", "2026-04-23T10:00:00Z"])
        .env("CLOCKIFY_API_KEY", "secret")
        .env("CFD_WORKSPACE", "w1")
        .env("CFD_BASE_URL", server.base_url())
        .output()
        .unwrap();
    assert!(!running.status.success());
    assert!(stderr(&running).contains("entry split requires a finished entry"));
}

#[test]
fn split_entry_rejects_invalid_boundaries_after_rounding() {
    let before = split_entry_error("2026-04-23T09:00:00Z", None);
    assert!(stderr(&before).contains("split time must be after entry start"));

    let after = split_entry_error("2026-04-23T11:00:00Z", None);
    assert!(stderr(&after).contains("split time must be before entry end"));

    let gap = split_entry_error("2026-04-23T10:45:00Z", Some("15m"));
    assert!(stderr(&gap).contains("new entry start must be before original entry end"));
}

fn split_entry_error(at: &str, gap: Option<&str>) -> std::process::Output {
    let server = TestServer::spawn(vec![MockResponse::ok(USER), MockResponse::ok(ENTRY)]);
    let mut command = bin();
    command.args(["split", "entry", "e1", "--at", at]);
    if let Some(gap) = gap {
        command.args(["--gap", gap]);
    }
    command
        .env("CLOCKIFY_API_KEY", "secret")
        .env("CFD_WORKSPACE", "w1")
        .env("CFD_BASE_URL", server.base_url())
        .output()
        .unwrap()
}

#[test]
fn split_entry_overlap_warning_combines_intervals_and_yes_skips_prompt() {
    let overlaps = r#"[{"id":"e1","workspaceId":"w1","userId":"u1","projectId":"p1","description":"Self","timeInterval":{"start":"2026-04-23T09:00:00Z","end":"2026-04-23T11:00:00Z"}},{"id":"a","workspaceId":"w1","userId":"u1","projectId":"p1","description":"A","timeInterval":{"start":"2026-04-23T09:30:00Z","end":"2026-04-23T09:45:00Z"}},{"id":"b","workspaceId":"w1","userId":"u1","projectId":"p1","description":"B","timeInterval":{"start":"2026-04-23T10:30:00Z","end":"2026-04-23T10:45:00Z"}}]"#;
    let server = TestServer::spawn(vec![
        MockResponse::ok(USER),
        MockResponse::ok(ENTRY),
        MockResponse::ok(overlaps),
        MockResponse::ok(overlaps),
        MockResponse::ok(UPDATED_ENTRY),
        MockResponse::ok(CREATED_ENTRY),
        MockResponse::ok(PROJECT),
        MockResponse::ok(PROJECT),
    ]);

    let output = bin()
        .args(["split", "entry", "e1", "--at", "2026-04-23T10:00:00Z", "-y"])
        .env("CLOCKIFY_API_KEY", "secret")
        .env("CFD_WORKSPACE", "w1")
        .env("CFD_BASE_URL", server.base_url())
        .output()
        .unwrap();

    assert!(output.status.success(), "{}", stderr(&output));
    assert!(stderr(&output).contains("warning: overlaps existing entries: a, b"));
    assert!(!stderr(&output).contains("Continue despite overlap?"));
}

#[test]
fn split_timer_stops_current_and_starts_new_timer() {
    let (_dir, config_path) = support::temp_config_path();
    fs::write(&config_path, "{\n  \"rounding\": \"15m\"\n}\n").unwrap();
    let server = TestServer::spawn(vec![
        MockResponse::ok(USER),
        MockResponse::ok(RUNNING),
        MockResponse::ok("[]"),
        MockResponse::ok("[]"),
        MockResponse::ok(STOPPED_TIMER),
        MockResponse::ok(CREATED_TIMER),
        MockResponse::ok(PROJECT),
        MockResponse::ok(PROJECT),
    ]);

    let output = bin()
        .args([
            "split",
            "timer",
            "--at",
            "2026-04-23T10:07:00Z",
            "--gap",
            "5m",
        ])
        .env("CFD_CONFIG", &config_path)
        .env("CLOCKIFY_API_KEY", "secret")
        .env("CFD_WORKSPACE", "w1")
        .env("CFD_BASE_URL", server.base_url())
        .output()
        .unwrap();

    assert!(output.status.success(), "{}", stderr(&output));
    let text = stdout(&output);
    assert!(text.contains("updated:\nid: timer1\n"));
    assert!(text.contains("created:\nid: timer2\n"));
    assert!(text.contains("end: \n"));

    let requests = server.requests();
    assert_eq!(requests[4].method, "PATCH");
    assert_eq!(requests[4].body, "{\"end\":\"2026-04-23T10:00:00+00:00\"}");
    assert_eq!(
        requests[5].body,
        "{\"description\":\"Focus\",\"projectId\":\"p1\",\"start\":\"2026-04-23T10:00:00+00:00\",\"tagIds\":[\"tag1\"],\"taskId\":\"t1\"}"
    );
}

#[test]
fn split_timer_json_no_rounding_uses_unrounded_gap() {
    let (_dir, config_path) = support::temp_config_path();
    fs::write(&config_path, "{\n  \"rounding\": \"15m\"\n}\n").unwrap();
    let stopped = r#"{"id":"timer1","workspaceId":"w1","userId":"u1","projectId":"p1","description":"Focus","timeInterval":{"start":"2026-04-23T09:00:00Z","end":"2026-04-23T10:07:00+00:00"}}"#;
    let created = r#"{"id":"timer2","workspaceId":"w1","userId":"u1","projectId":"p1","description":"Focus","timeInterval":{"start":"2026-04-23T10:12:00+00:00"}}"#;
    let server = TestServer::spawn(vec![
        MockResponse::ok(USER),
        MockResponse::ok(RUNNING),
        MockResponse::ok("[]"),
        MockResponse::ok("[]"),
        MockResponse::ok(stopped),
        MockResponse::ok(created),
    ]);

    let output = bin()
        .args([
            "split",
            "timer",
            "--at",
            "2026-04-23T10:07:00Z",
            "--gap",
            "5m",
            "--no-rounding",
            "--format",
            "raw",
        ])
        .env("CFD_CONFIG", &config_path)
        .env("CLOCKIFY_API_KEY", "secret")
        .env("CFD_WORKSPACE", "w1")
        .env("CFD_BASE_URL", server.base_url())
        .output()
        .unwrap();

    assert!(output.status.success(), "{}", stderr(&output));
    let value: Value = serde_json::from_str(&stdout(&output)).unwrap();
    assert_eq!(value["updated"]["id"], "timer1");
    assert_eq!(
        value["created"]["timeInterval"]["start"],
        "2026-04-23T10:12:00+00:00"
    );
}

#[test]
fn split_timer_rejects_missing_running_timer_project_and_boundaries() {
    let no_timer = TestServer::spawn(vec![MockResponse::ok(USER), MockResponse::ok("[]")]);
    let output = bin()
        .args(["split", "timer", "--at", "2026-04-23T10:00:00Z"])
        .env("CLOCKIFY_API_KEY", "secret")
        .env("CFD_WORKSPACE", "w1")
        .env("CFD_BASE_URL", no_timer.base_url())
        .output()
        .unwrap();
    assert!(!output.status.success());
    assert!(stderr(&output).contains("no running timer"));

    let no_project = TestServer::spawn(vec![
        MockResponse::ok(USER),
        MockResponse::ok(
            r#"[{"id":"timer1","workspaceId":"w1","userId":"u1","description":"Focus","timeInterval":{"start":"2026-04-23T09:00:00Z"}}]"#,
        ),
    ]);
    let output = bin()
        .args(["split", "timer", "--at", "2026-04-23T10:00:00Z"])
        .env("CLOCKIFY_API_KEY", "secret")
        .env("CFD_WORKSPACE", "w1")
        .env("CFD_BASE_URL", no_project.base_url())
        .output()
        .unwrap();
    assert!(!output.status.success());
    assert!(stderr(&output).contains("current timer has no project to split"));

    let at_start = split_timer_error("2026-04-23T09:00:00Z");
    assert!(stderr(&at_start).contains("split time must be after timer start"));

    let future = split_timer_error("2099-04-23T10:00:00Z");
    assert!(stderr(&future).contains("split time must not be in the future"));
}

fn split_timer_error(at: &str) -> std::process::Output {
    let server = TestServer::spawn(vec![MockResponse::ok(USER), MockResponse::ok(RUNNING)]);
    bin()
        .args(["split", "timer", "--at", at])
        .env("CLOCKIFY_API_KEY", "secret")
        .env("CFD_WORKSPACE", "w1")
        .env("CFD_BASE_URL", server.base_url())
        .output()
        .unwrap()
}

#[test]
fn split_timer_overlap_warning_combines_intervals_and_yes_skips_prompt() {
    let overlaps = r#"[{"id":"timer1","workspaceId":"w1","userId":"u1","projectId":"p1","description":"Self","timeInterval":{"start":"2026-04-23T09:00:00Z"}},{"id":"a","workspaceId":"w1","userId":"u1","projectId":"p1","description":"A","timeInterval":{"start":"2026-04-23T09:30:00Z","end":"2026-04-23T09:45:00Z"}},{"id":"b","workspaceId":"w1","userId":"u1","projectId":"p1","description":"B","timeInterval":{"start":"2026-04-23T10:30:00Z","end":"2026-04-23T10:45:00Z"}}]"#;
    let server = TestServer::spawn(vec![
        MockResponse::ok(USER),
        MockResponse::ok(RUNNING),
        MockResponse::ok(overlaps),
        MockResponse::ok(overlaps),
        MockResponse::ok(STOPPED_TIMER),
        MockResponse::ok(CREATED_TIMER),
        MockResponse::ok(PROJECT),
        MockResponse::ok(PROJECT),
    ]);

    let output = bin()
        .args(["split", "timer", "--at", "2026-04-23T10:00:00Z", "-y"])
        .env("CLOCKIFY_API_KEY", "secret")
        .env("CFD_WORKSPACE", "w1")
        .env("CFD_BASE_URL", server.base_url())
        .output()
        .unwrap();

    assert!(output.status.success(), "{}", stderr(&output));
    assert!(stderr(&output).contains("warning: overlaps existing entries: a, b"));
    assert!(!stderr(&output).contains("Continue despite overlap?"));
}

#[test]
fn split_timer_active_switch_updates_switched_entry_id() {
    let (_dir, config_path) = support::temp_config_path();
    fs::write(&config_path, active_switch_config()).unwrap();
    let created = r#"{"id":"timer2","workspaceId":"w1","userId":"u1","projectId":"p2","description":"Support call","timeInterval":{"start":"2026-05-06T10:30:00+00:00"}}"#;
    let stopped = r#"{"id":"b1","workspaceId":"w1","userId":"u1","projectId":"p2","description":"Support call","timeInterval":{"start":"2026-05-06T10:15:00Z","end":"2026-05-06T10:30:00+00:00"}}"#;
    let server = TestServer::spawn(vec![
        MockResponse::ok(USER),
        MockResponse::ok(
            r#"[{"id":"b1","workspaceId":"w1","userId":"u1","projectId":"p2","description":"Support call","timeInterval":{"start":"2026-05-06T10:15:00Z"}}]"#,
        ),
        MockResponse::ok("[]"),
        MockResponse::ok("[]"),
        MockResponse::ok(stopped),
        MockResponse::ok(created),
        MockResponse::ok(r#"{"id":"p2","name":"Support","workspaceId":"w1"}"#),
        MockResponse::ok(r#"{"id":"p2","name":"Support","workspaceId":"w1"}"#),
        MockResponse::ok(USER),
        MockResponse::ok(
            r#"[{"id":"timer2","workspaceId":"w1","userId":"u1","projectId":"p2","description":"Support call","timeInterval":{"start":"2026-05-06T10:30:00+00:00"}}]"#,
        ),
        MockResponse::ok(
            r#"{"id":"timer2","workspaceId":"w1","userId":"u1","projectId":"p2","description":"Support call","timeInterval":{"start":"2026-05-06T10:30:00+00:00","end":"2026-05-06T10:45:00+00:00"}}"#,
        ),
        MockResponse::ok(
            r#"{"id":"a2","workspaceId":"w1","userId":"u1","projectId":"p1","taskId":"t1","tagIds":["tag1"],"description":"Timer A","timeInterval":{"start":"2026-05-06T10:45:00+00:00"}}"#,
        ),
    ]);

    let output = bin()
        .args([
            "split",
            "timer",
            "--at",
            "2026-05-06T10:30:00Z",
            "--no-rounding",
        ])
        .env("CFD_CONFIG", &config_path)
        .env("CLOCKIFY_API_KEY", "secret")
        .env("CFD_WORKSPACE", "w1")
        .env("CFD_BASE_URL", server.base_url())
        .output()
        .unwrap();

    assert!(output.status.success(), "{}", stderr(&output));
    let config: Value = serde_json::from_str(&fs::read_to_string(&config_path).unwrap()).unwrap();
    assert_eq!(config["activeSwitch"]["switchedEntryId"], "timer2");
    assert_eq!(
        config["activeSwitch"]["switchedStart"],
        "2026-05-06T10:30:00+00:00"
    );
    assert_eq!(config["activeSwitch"]["originalEntryId"], "a1");
    assert_eq!(config["activeSwitch"]["returnTo"]["projectId"], "p1");

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
    let config: Value = serde_json::from_str(&fs::read_to_string(&config_path).unwrap()).unwrap();
    assert!(config.get("activeSwitch").is_none());
}

#[test]
fn unknown_split_variant_is_rejected_before_network() {
    let (_dir, config_path) = support::temp_config_path();
    let output = bin()
        .args(["split", "project", "--at", "2026-04-23T10:00:00Z"])
        .env("CFD_CONFIG", &config_path)
        .env("CLOCKIFY_API_KEY", "secret")
        .env("CFD_WORKSPACE", "w1")
        .output()
        .unwrap();

    assert!(!output.status.success());
    assert!(stderr(&output).contains("unknown command: cfd split project"));
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
}"#
}

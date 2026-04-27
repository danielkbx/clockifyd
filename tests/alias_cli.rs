mod support;

use std::fs;

use support::{bin, stderr, stdout, MockResponse, TestServer};

#[test]
fn alias_create_list_and_delete_round_trip() {
    let (_dir, config_path) = support::temp_config_path();
    let server = TestServer::spawn(vec![
        MockResponse::ok(r#"{"id":"p1","name":"Project One","workspaceId":"w1"}"#),
        MockResponse::ok(r#"{"id":"t1","name":"Task One","projectId":"p1"}"#),
        MockResponse::ok(r#"{"id":"p1","name":"Project One","workspaceId":"w1"}"#),
        MockResponse::ok(r#"{"id":"t1","name":"Task One","projectId":"p1"}"#),
    ]);

    let create = bin()
        .args([
            "alias",
            "create",
            "standup",
            "--project",
            "p1",
            "--task",
            "t1",
            "--description",
            "Daily standup",
        ])
        .env("CFD_CONFIG", &config_path)
        .env("CLOCKIFY_API_KEY", "secret")
        .env("CFD_WORKSPACE", "w1")
        .env("CFD_BASE_URL", server.base_url())
        .output()
        .unwrap();
    assert!(create.status.success());
    assert_eq!(stdout(&create), "standup\n");

    let config = fs::read_to_string(&config_path).unwrap();
    assert!(config.contains("\"aliases\""));
    assert!(config.contains("\"standup\""));
    assert!(config.contains("\"project\": \"p1\""));
    assert!(config.contains("\"task\": \"t1\""));
    assert!(config.contains("\"description\": \"Daily standup\""));

    let list = bin()
        .args(["alias", "list"])
        .env("CFD_CONFIG", &config_path)
        .env("CLOCKIFY_API_KEY", "secret")
        .env("CFD_WORKSPACE", "w1")
        .env("CFD_BASE_URL", server.base_url())
        .output()
        .unwrap();
    assert!(list.status.success());
    let list_stdout = stdout(&list);
    assert!(list_stdout.contains("standup\n"));
    assert!(list_stdout.contains("project: Project One (p1)"));
    assert!(list_stdout.contains("task: Task One (t1)"));
    assert!(list_stdout.contains("description: Daily standup"));

    let delete = bin()
        .args(["alias", "delete", "standup", "-y"])
        .env("CFD_CONFIG", &config_path)
        .output()
        .unwrap();
    assert!(delete.status.success());
    assert_eq!(stdout(&delete), "standup\n");
    assert!(!fs::read_to_string(&config_path)
        .unwrap()
        .contains("\"standup\""));
}

#[test]
fn runtime_alias_start_uses_stored_fields() {
    let (_dir, config_path) = support::temp_config_path();
    fs::write(
        &config_path,
        r#"{
  "aliases": {
    "standup": {
      "project": "p1",
      "task": "t1",
      "description": "Daily standup"
    }
  }
}
"#,
    )
    .unwrap();
    let server = TestServer::spawn(vec![
        MockResponse::ok(r#"{"id":"u1","name":"Ada","email":"ada@example.com"}"#),
        MockResponse::ok("[]"),
        MockResponse::ok("[]"),
        MockResponse::ok(
            r#"{"id":"e1","workspaceId":"w1","userId":"u1","projectId":"p1","taskId":"t1","description":"Daily standup","timeInterval":{"start":"2026-04-23T09:07:00+00:00"}}"#,
        ),
    ]);

    let output = bin()
        .args([
            "standup",
            "start",
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

    let requests = server.requests();
    assert_eq!(
        requests[3].body,
        "{\"description\":\"Daily standup\",\"projectId\":\"p1\",\"start\":\"2026-04-23T09:07:00+00:00\",\"taskId\":\"t1\"}"
    );
}

#[test]
fn runtime_alias_start_rejects_field_overrides() {
    let (_dir, config_path) = support::temp_config_path();
    fs::write(
        &config_path,
        r#"{
  "aliases": {
    "standup": {
      "project": "p1"
    }
  }
}
"#,
    )
    .unwrap();

    let output = bin()
        .args(["standup", "start", "--project", "p2"])
        .env("CFD_CONFIG", &config_path)
        .env("CLOCKIFY_API_KEY", "secret")
        .env("CFD_WORKSPACE", "w1")
        .output()
        .unwrap();

    assert!(!output.status.success());
    assert!(stderr(&output).contains("does not accept --project"));
}

mod support;

use support::{bin, stderr, stdout, MockResponse, TestServer};

#[test]
fn today_renders_ascii_table_and_total() {
    let server = TestServer::spawn(vec![
        MockResponse::ok(r#"{"id":"u1","name":"Ada","email":"ada@example.com"}"#),
        MockResponse::ok(
            r#"[{"id":"e1","workspaceId":"w1","userId":"u1","projectId":"p1","taskId":"t1","description":"Planning","timeInterval":{"start":"2026-04-27T09:00:00Z","end":"2026-04-27T10:15:00Z","duration":"PT1H15M"}}]"#,
        ),
        MockResponse::ok(r#"[{"id":"p1","name":"Project One","workspaceId":"w1"}]"#),
    ]);

    let output = bin()
        .args(["today"])
        .env("CLOCKIFY_API_KEY", "secret")
        .env("CFD_WORKSPACE", "w1")
        .env("CFD_BASE_URL", server.base_url())
        .output()
        .unwrap();

    assert!(output.status.success(), "{}", stderr(&output));
    let text = stdout(&output);
    assert!(text.contains("+"));
    assert!(text.contains("| Project     | Task | Description | Time"));
    assert!(text.contains("| Project One | t1   | Planning"));
    assert!(text.contains("1h15m"));
    assert!(text.contains("| Total"));

    let requests = server.requests();
    assert_eq!(requests[0].path, "/api/v1/user");
    assert!(requests[1]
        .path
        .contains("/api/v1/workspaces/w1/user/u1/time-entries?"));
    assert!(requests[1].path.contains("start=20"));
    assert!(requests[1].path.contains("end=20"));
    assert!(!requests[1].path.contains("today"));
    assert_eq!(requests[2].path, "/api/v1/workspaces/w1/projects");
}

#[test]
fn today_json_returns_raw_entries() {
    let server = TestServer::spawn(vec![
        MockResponse::ok(r#"{"id":"u1","name":"Ada","email":"ada@example.com"}"#),
        MockResponse::ok(
            r#"[{"id":"e1","workspaceId":"w1","userId":"u1","description":"Planning","timeInterval":{"start":"2026-04-27T09:00:00Z","end":"2026-04-27T10:15:00Z","duration":"PT1H15M"}}]"#,
        ),
    ]);

    let output = bin()
        .args(["today", "--format", "json"])
        .env("CLOCKIFY_API_KEY", "secret")
        .env("CFD_WORKSPACE", "w1")
        .env("CFD_BASE_URL", server.base_url())
        .output()
        .unwrap();

    assert!(output.status.success(), "{}", stderr(&output));
    let text = stdout(&output);
    assert!(text.contains("\"id\": \"e1\""));
    assert!(text.contains("\"timeInterval\""));
    assert!(!text.contains("Total"));
    assert!(!text.contains("+---"));
}

#[test]
fn today_raw_aliases_json() {
    let server = TestServer::spawn(vec![
        MockResponse::ok(r#"{"id":"u1","name":"Ada","email":"ada@example.com"}"#),
        MockResponse::ok("[]"),
    ]);

    let output = bin()
        .args(["today", "--format", "raw"])
        .env("CLOCKIFY_API_KEY", "secret")
        .env("CFD_WORKSPACE", "w1")
        .env("CFD_BASE_URL", server.base_url())
        .output()
        .unwrap();

    assert!(output.status.success(), "{}", stderr(&output));
    assert_eq!(stdout(&output), "[]\n");
}

#[test]
fn today_rejects_columns() {
    let output = bin()
        .args(["today", "--columns", "start,end"])
        .env("CLOCKIFY_API_KEY", "secret")
        .env("CFD_WORKSPACE", "w1")
        .output()
        .unwrap();

    assert!(!output.status.success());
    assert!(stderr(&output).contains("cfd today does not support --columns"));
}

#[test]
fn today_help_works() {
    let output = bin().args(["help", "today"]).output().unwrap();

    assert!(output.status.success());
    let text = stdout(&output);
    assert!(text.contains("cfd today"));
    assert!(text.contains("Project, Task, Description, Time, Duration"));
    assert!(text.contains("HH:MM-now"));
}

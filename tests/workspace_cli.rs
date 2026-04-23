mod support;

use support::{bin, stderr, stdout, MockResponse, TestServer};

#[test]
fn workspace_list_supports_text_json_and_no_meta() {
    let server = TestServer::spawn(vec![
        MockResponse::ok(r#"[{"id":"w1","name":"Engineering"}]"#),
        MockResponse::ok(r#"[{"id":"w1","name":"Engineering"}]"#),
        MockResponse::ok(r#"[{"id":"w1","name":"Engineering"}]"#),
    ]);

    let text = bin()
        .args(["workspace", "list"])
        .env("CLOCKIFY_API_KEY", "secret")
        .env("CFD_BASE_URL", server.base_url())
        .output()
        .unwrap();
    assert!(text.status.success());
    assert_eq!(stdout(&text), "id: w1\nname: Engineering\n");

    let json = bin()
        .args(["workspace", "list", "--format", "json"])
        .env("CLOCKIFY_API_KEY", "secret")
        .env("CFD_BASE_URL", server.base_url())
        .output()
        .unwrap();
    assert!(json.status.success());
    assert!(stdout(&json).contains("\"id\": \"w1\""));
    assert!(stdout(&json).contains("\"name\": \"Engineering\""));

    let no_meta = bin()
        .args(["workspace", "list", "--no-meta"])
        .env("CLOCKIFY_API_KEY", "secret")
        .env("CFD_BASE_URL", server.base_url())
        .output()
        .unwrap();
    assert!(no_meta.status.success());
    assert_eq!(stdout(&no_meta), "name: Engineering\n");
}

#[test]
fn workspace_get_supports_text_and_json_output() {
    let server = TestServer::spawn(vec![
        MockResponse::ok(r#"{"id":"w1","name":"Engineering"}"#),
        MockResponse::ok(r#"{"id":"w1","name":"Engineering"}"#),
    ]);

    let text = bin()
        .args(["workspace", "get", "w1"])
        .env("CLOCKIFY_API_KEY", "secret")
        .env("CFD_BASE_URL", server.base_url())
        .output()
        .unwrap();
    assert!(text.status.success());
    assert_eq!(stdout(&text), "id: w1\nname: Engineering\n");

    let json = bin()
        .args(["workspace", "get", "w1", "--format", "json"])
        .env("CLOCKIFY_API_KEY", "secret")
        .env("CFD_BASE_URL", server.base_url())
        .output()
        .unwrap();
    assert!(json.status.success());
    assert!(stdout(&json).contains("\"id\": \"w1\""));
}

#[test]
fn workspace_list_supports_columns_and_validation() {
    let server = TestServer::spawn(vec![MockResponse::ok(
        r#"[{"id":"w1","name":"Engineering"}]"#,
    )]);

    let columns = bin()
        .args(["workspace", "list", "--columns", "id,name"])
        .env("CLOCKIFY_API_KEY", "secret")
        .env("CFD_BASE_URL", server.base_url())
        .output()
        .unwrap();
    assert!(columns.status.success());
    assert_eq!(stdout(&columns), "w1\tEngineering\n");

    let missing = bin()
        .args(["workspace", "list", "--columns"])
        .env("CLOCKIFY_API_KEY", "secret")
        .output()
        .unwrap();
    assert!(!missing.status.success());
    assert!(stderr(&missing).contains("usage: cfd workspace list --columns <id,name,...>"));

    let conflict = bin()
        .args([
            "workspace",
            "list",
            "--columns",
            "id,name",
            "--format",
            "json",
        ])
        .env("CLOCKIFY_API_KEY", "secret")
        .output()
        .unwrap();
    assert!(!conflict.status.success());
    assert!(
        stderr(&conflict).contains("use either --columns <list> or --format <text|json>, not both")
    );
}

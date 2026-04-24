mod support;

use support::{bin, stderr, stdout, MockResponse, TestServer};

#[test]
fn metadata_commands_fail_clearly_when_workspace_is_missing() {
    let (_dir, config_path) = support::temp_config_path();

    let output = bin()
        .args(["project", "list"])
        .env("CFD_CONFIG", &config_path)
        .env("CLOCKIFY_API_KEY", "dummy-key")
        .output()
        .unwrap();

    assert!(!output.status.success());
    assert!(stderr(&output).contains("missing workspace"));
}

#[test]
fn project_list_supports_text_json_and_no_meta() {
    let server = TestServer::spawn(vec![
        MockResponse::ok(
            r#"[{"id":"p1","name":"Clockify CLI","clientId":"c1","workspaceId":"w1"}]"#,
        ),
        MockResponse::ok(r#"[{"id":"w1","name":"Engineering"}]"#),
        MockResponse::ok(
            r#"[{"id":"p1","name":"Clockify CLI","clientId":"c1","workspaceId":"w1"}]"#,
        ),
        MockResponse::ok(r#"[{"id":"w1","name":"Engineering"}]"#),
        MockResponse::ok(
            r#"[{"id":"p1","name":"Clockify CLI","clientId":"c1","workspaceId":"w1"}]"#,
        ),
        MockResponse::ok(r#"[{"id":"w1","name":"Engineering"}]"#),
    ]);

    let text = bin()
        .args(["project", "list"])
        .env("CLOCKIFY_API_KEY", "secret")
        .env("CFD_WORKSPACE", "w1")
        .env("CFD_BASE_URL", server.base_url())
        .output()
        .unwrap();
    assert!(text.status.success());
    assert_eq!(
        stdout(&text),
        "id: p1\nname: Clockify CLI\nworkspaceName: Engineering\n"
    );

    let json = bin()
        .args(["project", "list", "--format", "json"])
        .env("CLOCKIFY_API_KEY", "secret")
        .env("CFD_WORKSPACE", "w1")
        .env("CFD_BASE_URL", server.base_url())
        .output()
        .unwrap();
    assert!(json.status.success());
    assert!(stdout(&json).contains("\"clientId\": \"c1\""));

    let no_meta = bin()
        .args(["project", "list", "--no-meta"])
        .env("CLOCKIFY_API_KEY", "secret")
        .env("CFD_WORKSPACE", "w1")
        .env("CFD_BASE_URL", server.base_url())
        .output()
        .unwrap();
    assert!(no_meta.status.success());
    assert_eq!(
        stdout(&no_meta),
        "name: Clockify CLI\nworkspaceName: Engineering\n"
    );
}

#[test]
fn project_get_text_includes_more_than_list_row() {
    let server = TestServer::spawn(vec![
        MockResponse::ok(r#"{"id":"p1","name":"Clockify CLI","clientId":"c1","workspaceId":"w1"}"#),
        MockResponse::ok(r#"[{"id":"w1","name":"Engineering"}]"#),
    ]);

    let output = bin()
        .args(["project", "get", "p1"])
        .env("CLOCKIFY_API_KEY", "secret")
        .env("CFD_WORKSPACE", "w1")
        .env("CFD_BASE_URL", server.base_url())
        .output()
        .unwrap();
    assert!(output.status.success());
    assert_eq!(
        stdout(&output),
        "id: p1\nname: Clockify CLI\nworkspaceName: Engineering\nclientId: c1\nworkspaceId: w1\n"
    );
}

#[test]
fn tag_get_uses_workspace_override_flag() {
    let server = TestServer::spawn(vec![MockResponse::ok(r#"{"id":"tag1","name":"billable"}"#)]);

    let output = bin()
        .args(["tag", "get", "tag1", "--workspace", "w2"])
        .env("CLOCKIFY_API_KEY", "secret")
        .env("CFD_WORKSPACE", "w1")
        .env("CFD_BASE_URL", server.base_url())
        .output()
        .unwrap();
    assert!(output.status.success());
    assert_eq!(stdout(&output), "id: tag1\nname: billable\n");

    let requests = server.requests();
    assert_eq!(requests[0].path, "/api/v1/workspaces/w2/tags/tag1");
}

#[test]
fn metadata_lists_support_columns_and_validation() {
    let project_server = TestServer::spawn(vec![
        MockResponse::ok(
            r#"[{"id":"p1","name":"Clockify CLI","clientId":"c1","workspaceId":"w1"}]"#,
        ),
        MockResponse::ok(r#"[{"id":"w1","name":"Engineering"}]"#),
    ]);
    let project = bin()
        .args([
            "project",
            "list",
            "--columns",
            "id,name,client,workspaceId,workspaceName",
        ])
        .env("CLOCKIFY_API_KEY", "secret")
        .env("CFD_WORKSPACE", "w1")
        .env("CFD_BASE_URL", project_server.base_url())
        .output()
        .unwrap();
    assert!(project.status.success());
    assert_eq!(stdout(&project), "p1\tClockify CLI\tc1\tw1\tEngineering\n");

    let client_server = TestServer::spawn(vec![MockResponse::ok(r#"[{"id":"c1","name":"Acme"}]"#)]);
    let client = bin()
        .args(["client", "list", "--columns", "id,name"])
        .env("CLOCKIFY_API_KEY", "secret")
        .env("CFD_WORKSPACE", "w1")
        .env("CFD_BASE_URL", client_server.base_url())
        .output()
        .unwrap();
    assert!(client.status.success());
    assert_eq!(stdout(&client), "c1\tAcme\n");

    let tag_server = TestServer::spawn(vec![MockResponse::ok(
        r#"[{"id":"tag1","name":"billable"}]"#,
    )]);
    let tag = bin()
        .args(["tag", "list", "--columns", "id,name"])
        .env("CLOCKIFY_API_KEY", "secret")
        .env("CFD_WORKSPACE", "w1")
        .env("CFD_BASE_URL", tag_server.base_url())
        .output()
        .unwrap();
    assert!(tag.status.success());
    assert_eq!(stdout(&tag), "tag1\tbillable\n");

    let missing = bin()
        .args(["project", "list", "--columns"])
        .env("CLOCKIFY_API_KEY", "secret")
        .env("CFD_WORKSPACE", "w1")
        .output()
        .unwrap();
    assert!(!missing.status.success());
    assert!(stderr(&missing).contains(
        "usage: cfd project list --columns <id,name,client,workspaceId,workspaceName,...>"
    ));

    let conflict = bin()
        .args(["client", "list", "--columns", "id,name", "--format", "json"])
        .env("CLOCKIFY_API_KEY", "secret")
        .env("CFD_WORKSPACE", "w1")
        .output()
        .unwrap();
    assert!(!conflict.status.success());
    assert!(
        stderr(&conflict).contains("use either --columns <list> or --format <text|json>, not both")
    );
}

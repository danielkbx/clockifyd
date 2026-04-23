mod support;

use std::fs;

use support::{bin, stderr, stdout, MockResponse, TestServer};

#[test]
fn task_list_uses_config_project_default_and_no_meta() {
    let (_dir, config_path) = support::temp_config_path();
    fs::write(&config_path, "{\n  \"project\": \"p1\"\n}\n").unwrap();
    let server = TestServer::spawn(vec![MockResponse::ok(
        r#"[{"id":"t1","name":"ABC-1: Implement something nice","projectId":"p1"}]"#,
    )]);

    let output = bin()
        .args(["task", "list", "--no-meta"])
        .env("CFD_CONFIG", &config_path)
        .env("CLOCKIFY_API_KEY", "secret")
        .env("CFD_WORKSPACE", "w1")
        .env("CFD_BASE_URL", server.base_url())
        .output()
        .unwrap();
    assert!(output.status.success());
    assert_eq!(stdout(&output), "name: ABC-1: Implement something nice\n");

    let requests = server.requests();
    assert_eq!(requests[0].path, "/api/v1/workspaces/w1/projects/p1/tasks");
}

#[test]
fn task_create_prints_resource_id_only() {
    let server = TestServer::spawn(vec![MockResponse::ok(
        r#"{"id":"t1","name":"ABC-1: Implement something nice","projectId":"p1"}"#,
    )]);

    let output = bin()
        .args([
            "task",
            "create",
            "--project",
            "p1",
            "--name",
            "ABC-1: Implement something nice",
        ])
        .env("CLOCKIFY_API_KEY", "secret")
        .env("CFD_WORKSPACE", "w1")
        .env("CFD_BASE_URL", server.base_url())
        .output()
        .unwrap();
    assert!(output.status.success());
    assert_eq!(stdout(&output), "t1\n");

    let requests = server.requests();
    assert_eq!(
        requests[0].body,
        "{\"name\":\"ABC-1: Implement something nice\"}"
    );
}

#[test]
fn task_create_requires_name() {
    let output = bin()
        .args(["task", "create", "--project", "p1"])
        .env("CLOCKIFY_API_KEY", "secret")
        .env("CFD_WORKSPACE", "w1")
        .output()
        .unwrap();

    assert!(!output.status.success());
    assert!(stderr(&output).contains("usage: cfd task create"));
}

#[test]
fn task_list_supports_columns_and_validation() {
    let (_dir, config_path) = support::temp_config_path();
    fs::write(&config_path, "{\n  \"project\": \"p1\"\n}\n").unwrap();
    let server = TestServer::spawn(vec![MockResponse::ok(
        r#"[{"id":"t1","name":"ABC-1: Implement something nice","projectId":"p1"}]"#,
    )]);

    let columns = bin()
        .args(["task", "list", "--columns", "id,name,project"])
        .env("CFD_CONFIG", &config_path)
        .env("CLOCKIFY_API_KEY", "secret")
        .env("CFD_WORKSPACE", "w1")
        .env("CFD_BASE_URL", server.base_url())
        .output()
        .unwrap();
    assert!(columns.status.success());
    assert_eq!(
        stdout(&columns),
        "t1\tABC-1: Implement something nice\tp1\n"
    );

    let missing = bin()
        .args(["task", "list", "--columns"])
        .env("CFD_CONFIG", &config_path)
        .env("CLOCKIFY_API_KEY", "secret")
        .env("CFD_WORKSPACE", "w1")
        .output()
        .unwrap();
    assert!(!missing.status.success());
    assert!(stderr(&missing).contains("usage: cfd task list --columns <id,name,project,...>"));

    let conflict = bin()
        .args(["task", "list", "--columns", "id,name", "--format", "json"])
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

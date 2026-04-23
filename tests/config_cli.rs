mod support;

use std::io::Write;
use std::process::Stdio;

use support::{bin, stderr, stdout, temp_config_path, MockResponse, TestServer};

#[test]
fn unknown_command_is_reported_before_config_load() {
    let temp = tempfile::tempdir().unwrap();
    let missing_parent = temp.path().join("missing").join("config.json");

    let output = bin()
        .args(["wrkspace", "list"])
        .env("CFD_CONFIG", missing_parent)
        .output()
        .unwrap();

    assert!(!output.status.success());
    assert!(stderr(&output).contains("unknown command: cfd wrkspace list"));
    assert!(!stderr(&output).contains("config"));
}

#[test]
fn config_workspace_round_trip_works() {
    let (_dir, config_path) = temp_config_path();

    let set = bin()
        .args(["config", "set", "workspace", "ws1"])
        .env("CFD_CONFIG", &config_path)
        .output()
        .unwrap();
    assert!(set.status.success());

    let get = bin()
        .args(["config", "get", "workspace"])
        .env("CFD_CONFIG", &config_path)
        .output()
        .unwrap();
    assert!(get.status.success());
    assert_eq!(stdout(&get), "ws1\n");

    let unset = bin()
        .args(["config", "unset", "workspace"])
        .env("CFD_CONFIG", &config_path)
        .output()
        .unwrap();
    assert!(unset.status.success());
}

#[test]
fn config_rounding_and_project_round_trip_work() {
    let (_dir, config_path) = temp_config_path();

    let set_rounding = bin()
        .args(["config", "set", "rounding", "15m"])
        .env("CFD_CONFIG", &config_path)
        .output()
        .unwrap();
    assert!(set_rounding.status.success());

    let get_rounding = bin()
        .args(["config", "get", "rounding"])
        .env("CFD_CONFIG", &config_path)
        .output()
        .unwrap();
    assert!(get_rounding.status.success());
    assert_eq!(stdout(&get_rounding), "15m\n");

    let set_project = bin()
        .args(["config", "set", "project", "pr1"])
        .env("CFD_CONFIG", &config_path)
        .output()
        .unwrap();
    assert!(set_project.status.success());

    let get_project = bin()
        .args(["config", "get", "project"])
        .env("CFD_CONFIG", &config_path)
        .output()
        .unwrap();
    assert!(get_project.status.success());
    assert_eq!(stdout(&get_project), "pr1\n");
}

#[test]
fn invalid_usage_exits_non_zero() {
    let output = bin().args(["config", "set", "workspace"]).output().unwrap();

    assert!(!output.status.success());
    assert!(stderr(&output).contains("usage: cfd config set"));
}

#[test]
fn config_prints_full_masked_config() {
    let (_dir, config_path) = temp_config_path();

    std::fs::write(
        &config_path,
        "{\n  \"apiKey\": \"secret-key\",\n  \"workspace\": \"w2\",\n  \"project\": \"p1\",\n  \"rounding\": \"10m\"\n}\n",
    )
    .unwrap();

    let output = bin()
        .arg("config")
        .env("CFD_CONFIG", &config_path)
        .output()
        .unwrap();

    assert!(output.status.success());
    let config_stdout = stdout(&output);
    assert!(config_stdout.contains("apiKey: sec****key"));
    assert!(config_stdout.contains("workspace: w2"));
    assert!(config_stdout.contains("project: p1"));
    assert!(config_stdout.contains("rounding: 10m"));
}

#[test]
fn login_is_interactive_and_persists_selected_workspace_project_and_rounding() {
    let (_dir, config_path) = temp_config_path();
    let server = TestServer::spawn(vec![
        MockResponse::ok(r#"[{"id":"w1","name":"Engineering"},{"id":"w2","name":"Ops"}]"#),
        MockResponse::ok(r#"[{"id":"p1","name":"Platform"},{"id":"p2","name":"Billing"}]"#),
    ]);

    let mut child = bin()
        .arg("login")
        .env("CFD_CONFIG", &config_path)
        .env("CFD_BASE_URL", server.base_url())
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();

    child
        .stdin
        .as_mut()
        .unwrap()
        .write_all(b"secret-key\n2\n1\n4\n")
        .unwrap();

    let output = child.wait_with_output().unwrap();

    assert!(output.status.success());
    let login_stdout = stdout(&output);
    assert!(login_stdout.contains("Clockify API key: "));
    assert!(login_stdout.contains("Select default workspace:"));
    assert!(login_stdout.contains("2) Ops"));
    assert!(login_stdout.contains("Select default project:"));
    assert!(login_stdout.contains("1) Platform"));
    assert!(login_stdout.contains("Select default rounding:"));
    assert!(login_stdout.contains("4) 10m"));
    assert!(login_stdout.contains("Default workspace: w2\tOps"));
    assert!(login_stdout.contains("Default project: p1\tPlatform"));
    assert!(login_stdout.contains("Rounding: 10m"));

    let get_workspace = bin()
        .args(["config", "get", "workspace"])
        .env("CFD_CONFIG", &config_path)
        .output()
        .unwrap();
    assert!(get_workspace.status.success());
    assert_eq!(stdout(&get_workspace), "w2\n");

    let get_project = bin()
        .args(["config", "get", "project"])
        .env("CFD_CONFIG", &config_path)
        .output()
        .unwrap();
    assert!(get_project.status.success());
    assert_eq!(stdout(&get_project), "p1\n");

    let get_rounding = bin()
        .args(["config", "get", "rounding"])
        .env("CFD_CONFIG", &config_path)
        .output()
        .unwrap();
    assert!(get_rounding.status.success());
    assert_eq!(stdout(&get_rounding), "10m\n");
}

#[test]
fn config_interactive_reuses_api_key_and_updates_defaults() {
    let (_dir, config_path) = temp_config_path();
    std::fs::write(&config_path, "{\n  \"apiKey\": \"secret-key\"\n}\n").unwrap();
    let server = TestServer::spawn(vec![
        MockResponse::ok(r#"[{"id":"w1","name":"Engineering"},{"id":"w2","name":"Ops"}]"#),
        MockResponse::ok(r#"[{"id":"p1","name":"Platform"},{"id":"p2","name":"Billing"}]"#),
    ]);

    let mut child = bin()
        .args(["config", "interactive"])
        .env("CFD_CONFIG", &config_path)
        .env("CFD_BASE_URL", server.base_url())
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();

    child.stdin.as_mut().unwrap().write_all(b"1\n2\n5\n").unwrap();

    let output = child.wait_with_output().unwrap();

    assert!(output.status.success());
    let interactive_stdout = stdout(&output);
    assert!(!interactive_stdout.contains("Clockify API key: "));
    assert!(interactive_stdout.contains("Select default workspace:"));
    assert!(interactive_stdout.contains("1) Engineering"));
    assert!(interactive_stdout.contains("Select default project:"));
    assert!(interactive_stdout.contains("2) Billing"));
    assert!(interactive_stdout.contains("Select default rounding:"));
    assert!(interactive_stdout.contains("5) 15m"));
    assert!(interactive_stdout.contains("Saved config."));
    assert!(interactive_stdout.contains("Default workspace: w1\tEngineering"));
    assert!(interactive_stdout.contains("Default project: p2\tBilling"));
    assert!(interactive_stdout.contains("Rounding: 15m"));

    let config_output = bin()
        .arg("config")
        .env("CFD_CONFIG", &config_path)
        .output()
        .unwrap();
    let config_stdout = stdout(&config_output);
    assert!(config_stdout.contains("apiKey: sec****key"));
    assert!(config_stdout.contains("workspace: w1"));
    assert!(config_stdout.contains("project: p2"));
    assert!(config_stdout.contains("rounding: 15m"));
}

#[test]
fn config_interactive_uses_existing_values_as_defaults_when_present() {
    let (_dir, config_path) = temp_config_path();
    std::fs::write(
        &config_path,
        "{\n  \"apiKey\": \"secret-key\",\n  \"workspace\": \"w2\",\n  \"project\": \"p2\",\n  \"rounding\": \"10m\"\n}\n",
    )
    .unwrap();
    let server = TestServer::spawn(vec![
        MockResponse::ok(r#"[{"id":"w1","name":"Engineering"},{"id":"w2","name":"Ops"}]"#),
        MockResponse::ok(r#"[{"id":"p1","name":"Platform"},{"id":"p2","name":"Billing"}]"#),
    ]);

    let mut child = bin()
        .args(["config", "interactive"])
        .env("CFD_CONFIG", &config_path)
        .env("CFD_BASE_URL", server.base_url())
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();

    child.stdin.as_mut().unwrap().write_all(b"\n\n\n").unwrap();

    let output = child.wait_with_output().unwrap();

    assert!(output.status.success());
    let interactive_stdout = stdout(&output);
    assert!(interactive_stdout.contains("Default workspace [2]: "));
    assert!(interactive_stdout.contains("Default project [2]: "));
    assert!(interactive_stdout.contains("Default rounding [4]: "));
    assert!(interactive_stdout.contains("Default workspace: w2\tOps"));
    assert!(interactive_stdout.contains("Default project: p2\tBilling"));
    assert!(interactive_stdout.contains("Rounding: 10m"));
}

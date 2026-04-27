mod support;

use support::{bin, stderr, stdout, MockResponse, TestServer};

fn skill_output(args: &[&str]) -> std::process::Output {
    let temp = tempfile::tempdir().unwrap();
    let missing_config = temp.path().join("missing").join("config.json");

    bin()
        .args(args)
        .env("CFD_CONFIG", missing_config)
        .env_remove("CLOCKIFY_API_KEY")
        .env_remove("CFD_WORKSPACE")
        .output()
        .unwrap()
}

#[test]
fn global_help_explains_agent_skill_command() {
    let output = bin().arg("help").output().unwrap();

    assert!(output.status.success());
    let text = stdout(&output);
    assert!(text.contains("Agent Skills"));
    assert!(text.contains("skill"));
    assert!(text.contains("Print latest SKILL.md guidance for AI agents"));
    assert!(text.contains("AI agents can run `cfd skill`"));
    assert!(stderr(&output).is_empty());
}

#[test]
fn skill_help_is_available_from_both_entry_points() {
    let direct = bin().args(["help", "skill"]).output().unwrap();
    let nested = bin().args(["skill", "help"]).output().unwrap();

    assert!(direct.status.success());
    assert!(nested.status.success());
    assert_eq!(stdout(&direct), stdout(&nested));
    assert!(stdout(&direct).contains("Generate the latest SKILL.md content"));
    assert!(stdout(&direct).contains("Agents can run this command themselves"));
}

#[test]
fn skill_generates_standard_markdown_without_credentials() {
    let output = skill_output(&["skill"]);

    assert!(output.status.success());
    assert!(stderr(&output).is_empty());
    let text = stdout(&output);
    assert!(text.starts_with("---\n"));
    assert!(text.contains("name: clockify"));
    assert!(text.contains("time tracking"));
    assert!(text.contains("not generic issue tracker work logs"));
    assert!(text.contains("cfd --version"));
    assert!(text.contains("cfd skill --scope standard > SKILL.md"));
    assert!(text.contains("Prefer `--format json`"));
    assert!(text.contains("`--sort desc`"));
}

#[test]
fn skill_accepts_brief_scope_without_credentials() {
    let output = skill_output(&["skill", "--scope", "brief"]);

    assert!(output.status.success());
    assert!(stdout(&output).contains("cfd skill --scope brief > SKILL.md"));
    assert!(stderr(&output).is_empty());
}

#[test]
fn skill_accepts_format_md_without_credentials() {
    let output = skill_output(&["skill", "--format", "md"]);

    assert!(output.status.success());
    assert!(stdout(&output).starts_with("---\n"));
    assert!(stderr(&output).is_empty());
}

#[test]
fn skill_invalid_scope_errors_before_config_load() {
    let output = skill_output(&["skill", "--scope", "invalid"]);

    assert!(!output.status.success());
    assert_eq!(stdout(&output), "");
    assert_eq!(
        stderr(&output),
        "Invalid scope: invalid. Expected one of: brief, standard, full\n"
    );
}

#[test]
fn skill_rejects_json_format() {
    let output = skill_output(&["skill", "--format", "json"]);

    assert!(!output.status.success());
    assert_eq!(stdout(&output), "");
    assert_eq!(
        stderr(&output),
        "cfd skill only supports --format text or --format md\n"
    );
}

#[test]
fn workspace_skill_requires_api_key() {
    let output = skill_output(&["skill", "--workspace", "w1"]);

    assert!(!output.status.success());
    assert_eq!(stdout(&output), "");
    assert!(stderr(&output).contains("missing Clockify API key"));
}

#[test]
fn project_skill_requires_workspace_before_config_load() {
    let output = skill_output(&["skill", "--project", "p1"]);

    assert!(!output.status.success());
    assert_eq!(stdout(&output), "");
    assert_eq!(
        stderr(&output),
        "usage: cfd skill [--scope brief|standard|full] [--workspace <workspace-id> [--project <project-id>]]\n"
    );
}

#[test]
fn project_skill_rejects_missing_project_value() {
    let output = skill_output(&["skill", "--workspace", "w1", "--project"]);

    assert!(!output.status.success());
    assert_eq!(stdout(&output), "");
    assert_eq!(
        stderr(&output),
        "usage: cfd skill [--scope brief|standard|full] [--workspace <workspace-id> [--project <project-id>]]\n"
    );
}

#[test]
fn workspace_skill_resolves_workspace_and_includes_context() {
    let server = TestServer::spawn(vec![MockResponse::ok(
        r#"{"id":"w1","name":"Engineering"}"#,
    )]);

    let output = bin()
        .args(["skill", "--workspace", "w1", "--scope", "brief"])
        .env("CLOCKIFY_API_KEY", "test-key")
        .env("CFD_BASE_URL", server.base_url())
        .env_remove("CFD_WORKSPACE")
        .output()
        .unwrap();

    assert!(output.status.success(), "{}", stderr(&output));
    assert!(stderr(&output).is_empty());
    let text = stdout(&output);
    assert!(text.contains("name: clockify-engineering"));
    assert!(text.contains("## Workspace Context"));
    assert!(text.contains("- Name: Engineering"));
    assert!(text.contains("- ID: w1"));
    assert!(text.contains("cfd skill --workspace w1 --scope brief > SKILL.md"));
    assert!(text.contains("Clockify time tracking"));

    let requests = server.requests();
    assert_eq!(requests.len(), 1);
    assert_eq!(requests[0].method, "GET");
    assert_eq!(requests[0].path, "/api/v1/workspaces/w1");
}

#[test]
fn project_skill_resolves_workspace_and_project_and_includes_context() {
    let server = TestServer::spawn(vec![
        MockResponse::ok(r#"{"id":"w1","name":"Engineering"}"#),
        MockResponse::ok(r#"{"id":"p1","name":"Platform","workspaceId":"w1"}"#),
    ]);

    let output = bin()
        .args([
            "skill",
            "--workspace",
            "w1",
            "--project",
            "p1",
            "--scope",
            "standard",
        ])
        .env("CLOCKIFY_API_KEY", "test-key")
        .env("CFD_BASE_URL", server.base_url())
        .env_remove("CFD_WORKSPACE")
        .output()
        .unwrap();

    assert!(output.status.success(), "{}", stderr(&output));
    assert!(stderr(&output).is_empty());
    let text = stdout(&output);
    assert!(text.contains("name: clockify-platform"));
    assert!(text.contains("## Workspace Context"));
    assert!(text.contains("- Name: Engineering"));
    assert!(text.contains("- ID: w1"));
    assert!(text.contains("## Project Context"));
    assert!(text.contains("- Name: Platform"));
    assert!(text.contains("- ID: p1"));
    assert!(text.contains("cfd skill --workspace w1 --project p1 --scope standard > SKILL.md"));
    assert!(text.contains("cfd task list --workspace w1 --project p1 --format json"));
    assert!(text.contains(
        "cfd entry list --workspace w1 --start today --end today --format json --sort asc"
    ));
    assert!(text.contains("`--sort asc|desc`"));
    assert!(text.contains("cfd entry add --workspace w1 --start <iso> --duration <duration> --project p1 --description \"<work>\""));
    assert!(
        text.contains("cfd entry text list --workspace w1 --project p1 --columns text,lastUsed")
    );
    assert!(
        text.contains("cfd timer start \"ABC-1: Implement feature\" --workspace w1 --project p1")
    );

    let requests = server.requests();
    assert_eq!(requests.len(), 2);
    assert_eq!(requests[0].method, "GET");
    assert_eq!(requests[0].path, "/api/v1/workspaces/w1");
    assert_eq!(requests[1].method, "GET");
    assert_eq!(requests[1].path, "/api/v1/workspaces/w1/projects/p1");
}

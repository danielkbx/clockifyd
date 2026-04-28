mod support;

use chrono::{Datelike, Days, Local, LocalResult, TimeZone, Utc, Weekday};
use serde_json::Value;
use support::{bin, stderr, stdout, MockResponse, TestServer};

#[test]
fn status_renders_timer_today_week_and_totals() {
    let server = TestServer::spawn(vec![
        MockResponse::ok(r#"{"id":"u1","name":"Ada","email":"ada@example.com"}"#),
        MockResponse::ok(
            r#"[{"id":"run1","workspaceId":"w1","userId":"u1","projectId":"p1","taskId":"t1","description":"Planning","timeInterval":{"start":"2026-04-28T09:00:00Z"}}]"#,
        ),
        MockResponse::ok(
            r#"[{"id":"e1","workspaceId":"w1","userId":"u1","projectId":"p1","taskId":"t1","description":"Planning","timeInterval":{"start":"2026-04-28T09:00:00Z","end":"2026-04-28T10:00:00Z","duration":"PT1H"}},{"id":"e2","workspaceId":"w1","userId":"u1","projectId":"p1","description":"Review","timeInterval":{"start":"2026-04-28T10:00:00Z","end":"2026-04-28T10:35:00Z","duration":"PT35M"}}]"#,
        ),
        MockResponse::ok(
            r#"[{"id":"e1","workspaceId":"w1","userId":"u1","projectId":"p1","taskId":"t1","description":"Planning","timeInterval":{"start":"2026-04-28T09:00:00Z","end":"2026-04-28T10:00:00Z","duration":"PT1H"}},{"id":"e3","workspaceId":"w1","userId":"u1","projectId":"p2","description":"Support","timeInterval":{"start":"2026-04-27T10:00:00Z","end":"2026-04-27T11:20:00Z","duration":"PT1H20M"}}]"#,
        ),
        MockResponse::ok(
            r#"[{"id":"p1","name":"Project One","workspaceId":"w1"},{"id":"p2","name":"Project Two","workspaceId":"w1"}]"#,
        ),
    ]);

    let output = bin()
        .args(["status"])
        .env("CLOCKIFY_API_KEY", "secret")
        .env("CFD_WORKSPACE", "w1")
        .env("CFD_BASE_URL", server.base_url())
        .output()
        .unwrap();

    assert!(output.status.success(), "{}", stderr(&output));
    let text = stdout(&output);
    assert!(text.contains("Timer:\n  running: yes"));
    assert!(text.contains("| Project One | t1   | Planning"));
    assert!(text.contains("Today:"));
    assert!(text.contains("| Project     | Task | Description | Duration |"));
    assert!(text.contains("| Project One | t1   | Planning    | 1h"));
    assert!(text.contains("| Project One | none | Review      | 35m"));
    assert!(text.contains("Week:"));
    assert!(text.contains("| Project Two | none | Support     | 1h20m"));
    assert!(text.contains("| Total"));

    let requests = server.requests();
    assert_eq!(requests[0].path, "/api/v1/user");
    assert_eq!(
        requests[1].path,
        "/api/v1/workspaces/w1/time-entries/status/in-progress"
    );
    assert!(requests[2]
        .path
        .contains("/api/v1/workspaces/w1/user/u1/time-entries?"));
    assert!(requests[3]
        .path
        .contains("/api/v1/workspaces/w1/user/u1/time-entries?"));
    assert_eq!(requests[4].path, "/api/v1/workspaces/w1/projects");
}

#[test]
fn status_without_timer_or_entries_renders_empty_totals() {
    let server = TestServer::spawn(vec![
        MockResponse::ok(r#"{"id":"u1","name":"Ada","email":"ada@example.com"}"#),
        MockResponse::ok("[]"),
        MockResponse::ok("[]"),
        MockResponse::ok("[]"),
    ]);

    let output = bin()
        .args(["status"])
        .env("CLOCKIFY_API_KEY", "secret")
        .env("CFD_WORKSPACE", "w1")
        .env("CFD_BASE_URL", server.base_url())
        .output()
        .unwrap();

    assert!(output.status.success(), "{}", stderr(&output));
    assert_eq!(
        stdout(&output),
        "Timer:\n  running: no\n\nToday:\n  +---------+------+-------------+----------+\n  | Project | Task | Description | Duration |\n  +---------+------+-------------+----------+\n  +---------+------+-------------+----------+\n  | Total   |      |             | 0s       |\n  +---------+------+-------------+----------+\n\nWeek:\n  +---------+------+-------------+----------+\n  | Project | Task | Description | Duration |\n  +---------+------+-------------+----------+\n  +---------+------+-------------+----------+\n  | Total   |      |             | 0s       |\n  +---------+------+-------------+----------+\n\n"
    );
    assert_eq!(server.requests().len(), 4);
}

#[test]
fn status_ignores_running_timers_for_other_users() {
    let server = TestServer::spawn(vec![
        MockResponse::ok(r#"{"id":"u1","name":"Ada","email":"ada@example.com"}"#),
        MockResponse::ok(
            r#"[{"id":"other","workspaceId":"w1","userId":"u2","projectId":"p1","description":"Someone else","timeInterval":{"start":"2026-04-28T09:00:00Z"}}]"#,
        ),
        MockResponse::ok("[]"),
        MockResponse::ok("[]"),
    ]);

    let output = bin()
        .args(["status"])
        .env("CLOCKIFY_API_KEY", "secret")
        .env("CFD_WORKSPACE", "w1")
        .env("CFD_BASE_URL", server.base_url())
        .output()
        .unwrap();

    assert!(output.status.success(), "{}", stderr(&output));
    let text = stdout(&output);
    assert!(text.contains("Timer:\n  running: no"));
    assert!(!text.contains("Someone else"));
    assert_eq!(server.requests().len(), 4);
}

#[test]
fn status_groups_same_project_task_description() {
    let server = TestServer::spawn(vec![
        MockResponse::ok(r#"{"id":"u1","name":"Ada","email":"ada@example.com"}"#),
        MockResponse::ok("[]"),
        MockResponse::ok(
            r#"[{"id":"e1","workspaceId":"w1","userId":"u1","projectId":"p1","taskId":"t1","description":"Planning","timeInterval":{"start":"2026-04-28T09:00:00Z","end":"2026-04-28T10:00:00Z","duration":"PT1H"}},{"id":"e2","workspaceId":"w1","userId":"u1","projectId":"p1","taskId":"t1","description":"Planning","timeInterval":{"start":"2026-04-28T10:00:00Z","end":"2026-04-28T10:30:00Z","duration":"PT30M"}},{"id":"e3","workspaceId":"w1","userId":"u1","projectId":"p1","taskId":"t1","description":"Review","timeInterval":{"start":"2026-04-28T11:00:00Z","end":"2026-04-28T11:15:00Z","duration":"PT15M"}}]"#,
        ),
        MockResponse::ok("[]"),
        MockResponse::ok(r#"[{"id":"p1","name":"Project One","workspaceId":"w1"}]"#),
    ]);

    let output = bin()
        .args(["status"])
        .env("CLOCKIFY_API_KEY", "secret")
        .env("CFD_WORKSPACE", "w1")
        .env("CFD_BASE_URL", server.base_url())
        .output()
        .unwrap();

    assert!(output.status.success(), "{}", stderr(&output));
    let text = stdout(&output);
    assert!(text.contains("| Project One | t1   | Planning    | 1h30m"));
    assert!(text.contains("| Project One | t1   | Review      | 15m"));
}

#[test]
fn status_week_start_monday_sends_monday_boundaries() {
    let server = TestServer::spawn(empty_status_responses());
    let output = bin()
        .args(["status", "--week-start", "monday"])
        .env("CLOCKIFY_API_KEY", "secret")
        .env("CFD_WORKSPACE", "w1")
        .env("CFD_BASE_URL", server.base_url())
        .output()
        .unwrap();

    assert!(output.status.success(), "{}", stderr(&output));
    let requests = server.requests();
    assert_query_contains_bounds(&requests[3].path, week_bounds(Weekday::Mon));
}

#[test]
fn status_week_start_sunday_sends_sunday_boundaries() {
    let server = TestServer::spawn(empty_status_responses());
    let output = bin()
        .args(["status", "--week-start", "sunday"])
        .env("CLOCKIFY_API_KEY", "secret")
        .env("CFD_WORKSPACE", "w1")
        .env("CFD_BASE_URL", server.base_url())
        .output()
        .unwrap();

    assert!(output.status.success(), "{}", stderr(&output));
    let requests = server.requests();
    assert_query_contains_bounds(&requests[3].path, week_bounds(Weekday::Sun));
}

#[test]
fn status_rejects_invalid_week_start() {
    let output = bin()
        .args(["status", "--week-start", "friday"])
        .env("CLOCKIFY_API_KEY", "secret")
        .env("CFD_WORKSPACE", "w1")
        .output()
        .unwrap();

    assert!(!output.status.success());
    assert!(stderr(&output).contains("usage: cfd status [--week-start monday|sunday]"));
}

#[test]
fn status_json_returns_structured_summary() {
    let server = TestServer::spawn(vec![
        MockResponse::ok(r#"{"id":"u1","name":"Ada","email":"ada@example.com"}"#),
        MockResponse::ok("[]"),
        MockResponse::ok(
            r#"[{"id":"e1","workspaceId":"w1","userId":"u1","projectId":"p1","taskId":"t1","description":"Planning","timeInterval":{"start":"2026-04-28T09:00:00Z","end":"2026-04-28T10:30:00Z","duration":"PT1H30M"}}]"#,
        ),
        MockResponse::ok("[]"),
        MockResponse::ok(r#"[{"id":"p1","name":"Project One","workspaceId":"w1"}]"#),
    ]);

    let output = bin()
        .args(["status", "--format", "json"])
        .env("CLOCKIFY_API_KEY", "secret")
        .env("CFD_WORKSPACE", "w1")
        .env("CFD_BASE_URL", server.base_url())
        .output()
        .unwrap();

    assert!(output.status.success(), "{}", stderr(&output));
    let json: Value = serde_json::from_str(&stdout(&output)).unwrap();
    assert_eq!(json["timer"]["running"], false);
    assert!(json["timer"]["entry"].is_null());
    assert_eq!(json["today"]["groups"][0]["projectName"], "Project One");
    assert_eq!(json["today"]["groups"][0]["taskId"], "t1");
    assert_eq!(json["today"]["groups"][0]["description"], "Planning");
    assert_eq!(json["today"]["groups"][0]["durationSeconds"], 5400);
    assert_eq!(json["today"]["total"], "1h30m");
    assert_eq!(json["week"]["weekStart"], "monday");
    assert!(json.get("timeInterval").is_none());
}

#[test]
fn status_raw_aliases_json() {
    let server = TestServer::spawn(empty_status_responses());

    let output = bin()
        .args(["status", "--format", "raw"])
        .env("CLOCKIFY_API_KEY", "secret")
        .env("CFD_WORKSPACE", "w1")
        .env("CFD_BASE_URL", server.base_url())
        .output()
        .unwrap();

    assert!(output.status.success(), "{}", stderr(&output));
    let json: Value = serde_json::from_str(&stdout(&output)).unwrap();
    assert_eq!(json["timer"]["running"], false);
}

#[test]
fn status_rejects_columns() {
    let output = bin()
        .args(["status", "--columns", "project,duration"])
        .env("CLOCKIFY_API_KEY", "secret")
        .env("CFD_WORKSPACE", "w1")
        .output()
        .unwrap();

    assert!(!output.status.success());
    assert!(stderr(&output).contains("cfd status does not support --columns"));
}

#[test]
fn status_help_works() {
    let output = bin().args(["help", "status"]).output().unwrap();

    assert!(output.status.success());
    let text = stdout(&output);
    assert!(text.contains("cfd status"));
    assert!(text.contains("--week-start monday|sunday"));
    assert!(text.contains("project + task + description"));
}

fn empty_status_responses() -> Vec<MockResponse> {
    vec![
        MockResponse::ok(r#"{"id":"u1","name":"Ada","email":"ada@example.com"}"#),
        MockResponse::ok("[]"),
        MockResponse::ok("[]"),
        MockResponse::ok("[]"),
    ]
}

fn assert_query_contains_bounds(path: &str, (start, end): (String, String)) {
    assert!(
        path.contains(&format!("start={}", urlencoding::encode(&start))),
        "{path}"
    );
    assert!(
        path.contains(&format!("end={}", urlencoding::encode(&end))),
        "{path}"
    );
}

fn week_bounds(week_start: Weekday) -> (String, String) {
    let now = Local::now();
    let days_since_start =
        (7 + now.weekday().num_days_from_monday() - week_start.num_days_from_monday()) % 7;
    let start_date = now
        .date_naive()
        .checked_sub_days(Days::new(days_since_start.into()))
        .unwrap();
    let end_date = start_date.checked_add_days(Days::new(7)).unwrap();
    (local_midnight_utc(start_date), local_midnight_utc(end_date))
}

fn local_midnight_utc(date: chrono::NaiveDate) -> String {
    match Local.with_ymd_and_hms(date.year(), date.month(), date.day(), 0, 0, 0) {
        LocalResult::Single(value) => value.with_timezone(&Utc).to_rfc3339(),
        _ => panic!("failed to resolve local midnight"),
    }
}

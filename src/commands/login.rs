use std::io::{self, BufRead, Write};

use crate::args::ParsedArgs;
use crate::client::{ClockifyClient, HttpTransport, UreqTransport};
use crate::config::{get_config, save_config};
use crate::error::CfdError;
use crate::input::{prompt_line_with_io, select_index_with_io};
use crate::types::{Project, RoundingMode, Workspace};

pub fn execute(args: &ParsedArgs) -> Result<(), CfdError> {
    if !args.positional.is_empty() {
        return Err(CfdError::message("usage: cfd login"));
    }

    let stdin = io::stdin();
    let stdout = io::stdout();
    let mut reader = stdin.lock();
    let mut writer = stdout.lock();
    let api_key = prompt_api_key(&mut reader, &mut writer)?;
    let client = ClockifyClient::new(api_key.clone(), UreqTransport);

    run_setup_with_io(&mut reader, &mut writer, &client, &api_key, "Saved login.")
}

pub(crate) fn run_setup_with_io<R, W, T>(
    reader: &mut R,
    writer: &mut W,
    client: &ClockifyClient<T>,
    api_key: &str,
    saved_message: &str,
) -> Result<(), CfdError>
where
    R: BufRead,
    W: Write,
    T: HttpTransport,
{
    let existing_config = get_config()?;
    let workspaces = client.list_workspaces().map_err(map_login_error)?;
    let workspace = select_default_workspace(
        &workspaces,
        existing_config.workspace.as_deref(),
        reader,
        writer,
    )?;
    let project = match workspace.as_ref() {
        Some(workspace) => {
            let projects = client
                .list_projects(&workspace.id)
                .map_err(map_login_error)?;
            select_default_project(
                &projects,
                existing_config.project.as_deref(),
                reader,
                writer,
            )?
        }
        None => None,
    };
    let rounding = select_default_rounding(existing_config.rounding, reader, writer)?;

    let mut config = existing_config;
    config.api_key = Some(api_key.to_owned());
    config.workspace = workspace.as_ref().map(|item| item.id.clone());
    config.project = project.as_ref().map(|item| item.id.clone());
    config.rounding = rounding;
    save_config(&config)?;

    writeln!(writer, "{saved_message}")?;
    match workspace {
        Some(workspace) => writeln!(
            writer,
            "Default workspace: {}\t{}",
            workspace.id, workspace.name
        )?,
        None => writeln!(writer, "Default workspace: none")?,
    }
    match project {
        Some(project) => writeln!(writer, "Default project: {}\t{}", project.id, project.name)?,
        None => writeln!(writer, "Default project: none")?,
    }
    writeln!(
        writer,
        "Rounding: {}",
        rounding.map(rounding_label).unwrap_or("none")
    )?;

    Ok(())
}

fn prompt_api_key(reader: &mut dyn BufRead, writer: &mut dyn Write) -> Result<String, CfdError> {
    let api_key = prompt_line_with_io("Clockify API key: ", reader, writer)?;
    if api_key.is_empty() {
        return Err(CfdError::message("API key cannot be empty"));
    }
    Ok(api_key)
}

fn run_with_io<R, W, T>(reader: &mut R, writer: &mut W, transport: T) -> Result<(), CfdError>
where
    R: BufRead,
    W: Write,
    T: HttpTransport,
{
    let api_key = prompt_api_key(reader, writer)?;
    let client = ClockifyClient::new(api_key.clone(), transport);
    run_setup_with_io(reader, writer, &client, &api_key, "Saved login.")
}

fn select_default_workspace(
    workspaces: &[Workspace],
    current_workspace_id: Option<&str>,
    reader: &mut dyn BufRead,
    writer: &mut dyn Write,
) -> Result<Option<Workspace>, CfdError> {
    if workspaces.is_empty() {
        writeln!(
            writer,
            "No workspaces available; default workspace set to none."
        )?;
        return Ok(None);
    }

    writeln!(writer, "Select default workspace:")?;
    writeln!(writer, "  0) none")?;
    for (index, workspace) in workspaces.iter().enumerate() {
        writeln!(writer, "  {}) {}", index + 1, workspace.name)?;
    }

    let default_index = workspaces
        .iter()
        .position(|workspace| Some(workspace.id.as_str()) == current_workspace_id)
        .map(|index| index + 1)
        .unwrap_or(0);
    let prompt = format!("Default workspace [{default_index}]: ");
    let selection = select_index_with_io(&prompt, workspaces.len(), default_index, reader, writer)?;
    if selection == 0 {
        Ok(None)
    } else {
        Ok(Some(workspaces[selection - 1].clone()))
    }
}

fn select_default_project(
    projects: &[Project],
    current_project_id: Option<&str>,
    reader: &mut dyn BufRead,
    writer: &mut dyn Write,
) -> Result<Option<Project>, CfdError> {
    if projects.is_empty() {
        writeln!(
            writer,
            "No projects available; default project set to none."
        )?;
        return Ok(None);
    }

    writeln!(writer, "Select default project:")?;
    writeln!(writer, "  0) none")?;
    for (index, project) in projects.iter().enumerate() {
        writeln!(writer, "  {}) {}", index + 1, project.name)?;
    }

    let default_index = projects
        .iter()
        .position(|project| Some(project.id.as_str()) == current_project_id)
        .map(|index| index + 1)
        .unwrap_or(0);
    let prompt = format!("Default project [{default_index}]: ");
    let selection = select_index_with_io(&prompt, projects.len(), default_index, reader, writer)?;
    if selection == 0 {
        Ok(None)
    } else {
        Ok(Some(projects[selection - 1].clone()))
    }
}

fn select_default_rounding(
    current_rounding: Option<RoundingMode>,
    reader: &mut dyn BufRead,
    writer: &mut dyn Write,
) -> Result<Option<RoundingMode>, CfdError> {
    let options = [
        ("off", RoundingMode::Off),
        ("1m", RoundingMode::OneMinute),
        ("5m", RoundingMode::FiveMinutes),
        ("10m", RoundingMode::TenMinutes),
        ("15m", RoundingMode::FifteenMinutes),
    ];

    writeln!(writer, "Select default rounding:")?;
    writeln!(writer, "  0) none")?;
    for (index, (label, _)) in options.iter().enumerate() {
        writeln!(writer, "  {}) {}", index + 1, label)?;
    }

    let default_index = options
        .iter()
        .position(|(_, mode)| Some(*mode) == current_rounding)
        .map(|index| index + 1)
        .unwrap_or(0);
    let prompt = format!("Default rounding [{default_index}]: ");
    let selection = select_index_with_io(&prompt, options.len(), default_index, reader, writer)?;
    if selection == 0 {
        Ok(None)
    } else {
        Ok(Some(options[selection - 1].1))
    }
}

fn rounding_label(mode: RoundingMode) -> &'static str {
    match mode {
        RoundingMode::Off => "off",
        RoundingMode::OneMinute => "1m",
        RoundingMode::FiveMinutes => "5m",
        RoundingMode::TenMinutes => "10m",
        RoundingMode::FifteenMinutes => "15m",
    }
}

fn map_login_error(error: CfdError) -> CfdError {
    match error {
        CfdError::HttpStatus { status: 401 | 403 } => {
            CfdError::message("Could not authenticate. Check API key.")
        }
        other => other,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::test_env_mutex;
    use crate::error::CfdError;
    use crate::types::StoredConfig;
    use std::cell::RefCell;

    struct MockTransport {
        bodies: RefCell<Vec<String>>,
        error: Option<CfdError>,
    }

    impl MockTransport {
        fn ok(bodies: &[&str]) -> Self {
            Self {
                bodies: RefCell::new(bodies.iter().map(|body| (*body).to_owned()).collect()),
                error: None,
            }
        }

        fn err(error: CfdError) -> Self {
            Self {
                bodies: RefCell::new(Vec::new()),
                error: Some(error),
            }
        }
    }

    impl HttpTransport for MockTransport {
        fn get(&self, _url: &str, _api_key: &str) -> Result<String, CfdError> {
            if let Some(error) = &self.error {
                return match error {
                    CfdError::Message(message) => Err(CfdError::message(message.clone())),
                    CfdError::HttpStatus { status } => {
                        Err(CfdError::HttpStatus { status: *status })
                    }
                    CfdError::Transport { message } => Err(CfdError::transport(message.clone())),
                    CfdError::Io(io_error) => Err(CfdError::transport(io_error.to_string())),
                    CfdError::Json(json_error) => Err(CfdError::transport(json_error.to_string())),
                };
            }

            let mut bodies = self.bodies.borrow_mut();
            if bodies.is_empty() {
                return Err(CfdError::message("unexpected get"));
            }

            Ok(bodies.remove(0))
        }

        fn post(&self, _url: &str, _api_key: &str, _body: &str) -> Result<String, CfdError> {
            Err(CfdError::message("unexpected post"))
        }

        fn put(&self, _url: &str, _api_key: &str, _body: &str) -> Result<String, CfdError> {
            Err(CfdError::message("unexpected put"))
        }

        fn patch(&self, _url: &str, _api_key: &str, _body: &str) -> Result<String, CfdError> {
            Err(CfdError::message("unexpected patch"))
        }

        fn delete(&self, _url: &str, _api_key: &str) -> Result<(), CfdError> {
            Err(CfdError::message("unexpected delete"))
        }
    }

    #[test]
    fn login_saves_api_key_and_selected_workspace_project_and_rounding() {
        let _lock = test_env_mutex().lock().unwrap();
        let dir = tempfile::tempdir().unwrap();
        unsafe { std::env::set_var("CFD_CONFIG", dir.path().join("config.json")) };

        let mut reader = io::Cursor::new("secret-key\n2\n1\n4\n");
        let mut writer = Vec::new();

        run_with_io(
            &mut reader,
            &mut writer,
            MockTransport::ok(&[
                r#"[{"id":"w1","name":"Engineering"},{"id":"w2","name":"Ops"}]"#,
                r#"[{"id":"p1","name":"Platform"},{"id":"p2","name":"Billing"}]"#,
            ]),
        )
        .unwrap();

        let config = get_config().unwrap();
        let output = String::from_utf8(writer).unwrap();

        assert_eq!(
            config,
            StoredConfig {
                api_key: Some("secret-key".into()),
                workspace: Some("w2".into()),
                rounding: Some(RoundingMode::TenMinutes),
                project: Some("p1".into()),
            }
        );
        assert!(output.contains("Default workspace: w2\tOps"));
        assert!(output.contains("Default project: p1\tPlatform"));
        assert!(output.contains("Rounding: 10m"));

        unsafe { std::env::remove_var("CFD_CONFIG") };
    }

    #[test]
    fn login_allows_skipping_workspace_project_and_rounding() {
        let _lock = test_env_mutex().lock().unwrap();
        let dir = tempfile::tempdir().unwrap();
        unsafe { std::env::set_var("CFD_CONFIG", dir.path().join("config.json")) };

        let mut reader = io::Cursor::new("secret-key\n0\n0\n");
        let mut writer = Vec::new();

        run_with_io(
            &mut reader,
            &mut writer,
            MockTransport::ok(&[r#"[{"id":"w1","name":"Engineering"}]"#]),
        )
        .unwrap();

        let config = get_config().unwrap();
        let output = String::from_utf8(writer).unwrap();

        assert_eq!(config.api_key.as_deref(), Some("secret-key"));
        assert_eq!(config.workspace, None);
        assert_eq!(config.project, None);
        assert_eq!(config.rounding, None);
        assert!(output.contains("Default workspace: none"));
        assert!(output.contains("Default project: none"));
        assert!(output.contains("Rounding: none"));

        unsafe { std::env::remove_var("CFD_CONFIG") };
    }

    #[test]
    fn login_allows_skipping_project_with_selected_workspace() {
        let _lock = test_env_mutex().lock().unwrap();
        let dir = tempfile::tempdir().unwrap();
        unsafe { std::env::set_var("CFD_CONFIG", dir.path().join("config.json")) };

        let mut reader = io::Cursor::new("secret-key\n1\n0\n1\n");
        let mut writer = Vec::new();

        run_with_io(
            &mut reader,
            &mut writer,
            MockTransport::ok(&[
                r#"[{"id":"w1","name":"Engineering"}]"#,
                r#"[{"id":"p1","name":"Platform"}]"#,
            ]),
        )
        .unwrap();

        let config = get_config().unwrap();
        let output = String::from_utf8(writer).unwrap();

        assert_eq!(config.workspace.as_deref(), Some("w1"));
        assert_eq!(config.project, None);
        assert_eq!(config.rounding, Some(RoundingMode::Off));
        assert!(output.contains("Default project: none"));
        assert!(output.contains("Rounding: off"));

        unsafe { std::env::remove_var("CFD_CONFIG") };
    }

    #[test]
    fn login_uses_existing_config_values_as_defaults() {
        let _lock = test_env_mutex().lock().unwrap();
        let dir = tempfile::tempdir().unwrap();
        unsafe { std::env::set_var("CFD_CONFIG", dir.path().join("config.json")) };

        save_config(&StoredConfig {
            api_key: Some("old-key".into()),
            workspace: Some("w2".into()),
            project: Some("p2".into()),
            rounding: Some(RoundingMode::TenMinutes),
        })
        .unwrap();

        let mut reader = io::Cursor::new("secret-key\n\n\n\n");
        let mut writer = Vec::new();

        run_with_io(
            &mut reader,
            &mut writer,
            MockTransport::ok(&[
                r#"[{"id":"w1","name":"Engineering"},{"id":"w2","name":"Ops"}]"#,
                r#"[{"id":"p1","name":"Platform"},{"id":"p2","name":"Billing"}]"#,
            ]),
        )
        .unwrap();

        let config = get_config().unwrap();
        let output = String::from_utf8(writer).unwrap();

        assert_eq!(config.api_key.as_deref(), Some("secret-key"));
        assert_eq!(config.workspace.as_deref(), Some("w2"));
        assert_eq!(config.project.as_deref(), Some("p2"));
        assert_eq!(config.rounding, Some(RoundingMode::TenMinutes));
        assert!(output.contains("Default workspace [2]: "));
        assert!(output.contains("Default project [2]: "));
        assert!(output.contains("Default rounding [4]: "));

        unsafe { std::env::remove_var("CFD_CONFIG") };
    }

    #[test]
    fn login_rejects_empty_api_key() {
        let mut reader = io::Cursor::new("\n");
        let mut writer = Vec::new();

        let error = run_with_io(&mut reader, &mut writer, MockTransport::ok(&["[]"]))
            .unwrap_err()
            .to_string();

        assert!(error.contains("API key cannot be empty"));
    }

    #[test]
    fn login_maps_auth_failures_to_clear_message() {
        let mut reader = io::Cursor::new("secret-key\n");
        let mut writer = Vec::new();

        let error = run_with_io(
            &mut reader,
            &mut writer,
            MockTransport::err(CfdError::HttpStatus { status: 401 }),
        )
        .unwrap_err()
        .to_string();

        assert_eq!(error, "Could not authenticate. Check API key.");
    }
}

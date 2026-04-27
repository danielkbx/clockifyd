mod args;
mod cli_spec;
mod client;
mod commands;
mod completion;
mod config;
mod datetime;
mod duration;
mod error;
mod format;
mod help;
mod input;
mod types;

use std::process::ExitCode;

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("{error}");
            ExitCode::from(1)
        }
    }
}

fn run() -> Result<(), error::CfdError> {
    let argv: Vec<String> = std::env::args().skip(1).collect();
    let args = args::parse_args(&argv);

    if args.flags.contains_key("version") {
        println!("cfd {}", env!("CARGO_PKG_VERSION"));
        return Ok(());
    }

    if args.resource.is_none() || args.resource.as_deref() == Some("help") {
        println!(
            "{}",
            help::render_help(args.action.as_deref(), args.subaction.as_deref(), None)
        );
        return Ok(());
    }

    if args.action.as_deref() == Some("help") {
        println!(
            "{}",
            help::render_help(
                args.resource.as_deref(),
                args.subaction.as_deref(),
                args.positional.first().map(String::as_str)
            )
        );
        return Ok(());
    }

    if matches!(
        (
            args.resource.as_deref(),
            args.action.as_deref(),
            args.subaction.as_deref()
        ),
        (Some("entry"), Some("text"), Some("help"))
    ) {
        println!("{}", help::render_help(Some("entry"), Some("text"), None));
        return Ok(());
    }

    let resource = args.resource.as_deref().unwrap_or_default();
    let action = args.action.as_deref();
    let subaction = args.subaction.as_deref();

    if let ("completion", Some(shell), None) = (resource, action, subaction) {
        if !args.positional.is_empty() {
            return Err(error::CfdError::message(format!(
                "unknown command: cfd completion {shell} {}",
                args.positional.join(" ")
            )));
        }
        let script = completion::render_completion(shell, &cli_spec::cli_spec())?;
        print!("{script}");
        if !script.ends_with('\n') {
            println!();
        }
        return Ok(());
    }

    let runtime_alias = if is_known_command(resource, action, subaction) {
        None
    } else if matches!((action, subaction), (Some("start"), None)) {
        let config = config::get_config()?;
        config.aliases.get(resource).cloned()
    } else {
        None
    };

    if runtime_alias.is_none() && !is_known_command(resource, action, subaction) {
        return Err(error::CfdError::message(format!(
            "unknown command: cfd {}{}{}",
            resource,
            action.map(|value| format!(" {value}")).unwrap_or_default(),
            subaction
                .map(|value| format!(" {value}"))
                .unwrap_or_default()
        )));
    }

    if resource == "skill" {
        commands::skill::validate(&args)?;
        if commands::skill::workspace_ref(&args)?.is_none()
            && commands::skill::project_ref(&args)?.is_none()
        {
            return commands::skill::run(None, None, &args);
        }
    }

    match (resource, action, subaction) {
        ("login", _, _) => commands::login::execute(&args),
        ("logout", _, _) => commands::logout::execute(),
        ("whoami", _, _) => {
            let config = config::get_config()?;
            let api_key = config::resolve_api_key(&config)?;
            let client = client::ClockifyClient::new(api_key, client::UreqTransport);
            commands::whoami::execute(&client, &args.output)
        }
        ("workspace", _, _) => {
            let config = config::get_config()?;
            let api_key = config::resolve_api_key(&config)?;
            let client = client::ClockifyClient::new(api_key, client::UreqTransport);
            commands::workspace::execute(&client, &args)
        }
        ("config", _, _) => commands::config::execute(&args),
        ("alias", Some("delete"), _) => commands::alias::execute_config_only(&args),
        ("project", _, _) => {
            let config = config::get_config()?;
            let api_key = config::resolve_api_key(&config)?;
            let workspace_id = config::resolve_workspace(args.workspace.as_deref(), &config)?;
            let client = client::ClockifyClient::new(api_key, client::UreqTransport);
            commands::project::execute(&client, &args, &workspace_id)
        }
        ("client", _, _) => {
            let config = config::get_config()?;
            let api_key = config::resolve_api_key(&config)?;
            let workspace_id = config::resolve_workspace(args.workspace.as_deref(), &config)?;
            let client = client::ClockifyClient::new(api_key, client::UreqTransport);
            commands::client::execute(&client, &args, &workspace_id)
        }
        ("tag", _, _) => {
            let config = config::get_config()?;
            let api_key = config::resolve_api_key(&config)?;
            let workspace_id = config::resolve_workspace(args.workspace.as_deref(), &config)?;
            let client = client::ClockifyClient::new(api_key, client::UreqTransport);
            commands::tag::execute(&client, &args, &workspace_id)
        }
        ("task", _, _) => {
            let config = config::get_config()?;
            let api_key = config::resolve_api_key(&config)?;
            let workspace_id = config::resolve_workspace(args.workspace.as_deref(), &config)?;
            let client = client::ClockifyClient::new(api_key, client::UreqTransport);
            commands::task::execute(&client, &args, &workspace_id, &config)
        }
        ("entry", Some("text"), Some("list")) => {
            let config = config::get_config()?;
            let api_key = config::resolve_api_key(&config)?;
            let workspace_id = config::resolve_workspace(args.workspace.as_deref(), &config)?;
            let client = client::ClockifyClient::new(api_key, client::UreqTransport);
            commands::entry::execute(&client, &args, &workspace_id, &config)
        }
        ("entry", _, _) => {
            let config = config::get_config()?;
            let api_key = config::resolve_api_key(&config)?;
            let workspace_id = config::resolve_workspace(args.workspace.as_deref(), &config)?;
            let client = client::ClockifyClient::new(api_key, client::UreqTransport);
            commands::entry::execute(&client, &args, &workspace_id, &config)
        }
        ("today", _, _) => {
            let config = config::get_config()?;
            let api_key = config::resolve_api_key(&config)?;
            let workspace_id = config::resolve_workspace(args.workspace.as_deref(), &config)?;
            let client = client::ClockifyClient::new(api_key, client::UreqTransport);
            commands::today::execute(&client, &args, &workspace_id)
        }
        ("timer", _, _) => {
            let config = config::get_config()?;
            let api_key = config::resolve_api_key(&config)?;
            let workspace_id = config::resolve_workspace(args.workspace.as_deref(), &config)?;
            let client = client::ClockifyClient::new(api_key, client::UreqTransport);
            commands::timer::execute(&client, &args, &workspace_id, &config)
        }
        ("alias", _, _) => {
            let config = config::get_config()?;
            let api_key = config::resolve_api_key(&config)?;
            let workspace_id = config::resolve_workspace(args.workspace.as_deref(), &config)?;
            let client = client::ClockifyClient::new(api_key, client::UreqTransport);
            commands::alias::execute(&client, &args, &workspace_id, &args.output)
        }
        ("skill", _, _) => {
            let config = config::get_config()?;
            let api_key = config::resolve_api_key(&config)?;
            let client = client::ClockifyClient::new(api_key, client::UreqTransport);
            let workspace_id = commands::skill::workspace_ref(&args)?;
            let workspace = workspace_id
                .map(|workspace_id| client.get_workspace(workspace_id))
                .transpose()?
                .map(commands::skill::SkillWorkspaceContext::from);
            let project = match (workspace_id, commands::skill::project_ref(&args)?) {
                (Some(workspace_id), Some(project_id)) => Some(
                    client
                        .get_project(workspace_id, project_id)
                        .map(commands::skill::SkillProjectContext::from)?,
                ),
                _ => None,
            };
            commands::skill::run(workspace, project, &args)
        }
        _ if runtime_alias.is_some() => {
            let config = config::get_config()?;
            let api_key = config::resolve_api_key(&config)?;
            let workspace_id = config::resolve_workspace(args.workspace.as_deref(), &config)?;
            let client = client::ClockifyClient::new(api_key, client::UreqTransport);
            commands::alias::execute_runtime_start(
                &client,
                resource,
                runtime_alias.as_ref().unwrap(),
                &args,
                &workspace_id,
                &config,
            )
        }
        _ => Err(error::CfdError::message(format!(
            "unknown command: cfd {}",
            resource
        ))),
    }
}

fn is_known_command(resource: &str, action: Option<&str>, subaction: Option<&str>) -> bool {
    matches!(
        (resource, action, subaction),
        ("login", None, None)
            | ("logout", None, None)
            | ("skill", None, None)
            | ("whoami", None, None)
            | ("workspace", Some("list" | "get"), None)
            | ("config", None, None)
            | (
                "config",
                Some("interactive" | "set" | "get" | "unset"),
                None
            )
            | ("alias", Some("create" | "list" | "delete"), None)
            | ("project", Some("list" | "get"), None)
            | ("client", Some("list" | "get"), None)
            | ("tag", Some("list" | "get"), None)
            | ("task", Some("list" | "get" | "create"), None)
            | (
                "entry",
                Some("list" | "get" | "add" | "update" | "delete"),
                None
            )
            | ("entry", Some("text"), Some("list"))
            | ("today", None, None)
            | ("timer", Some("current" | "start" | "stop"), None)
            | ("completion", Some("bash" | "zsh" | "fish"), None)
    )
}

#[cfg(test)]
mod main {
    use super::is_known_command;

    fn router_parts<'a>(path: &'a [&'a str]) -> (&'a str, Option<&'a str>, Option<&'a str>) {
        let resource = path[0];
        let action = path.get(1).copied();
        let subaction = if matches!((resource, action), ("entry", Some("text"))) {
            path.get(2).copied()
        } else {
            None
        };

        (resource, action, subaction)
    }

    #[test]
    fn known_commands_cover_entry_text_branch() {
        assert!(is_known_command("entry", Some("text"), Some("list")));
        assert!(is_known_command("today", None, None));
        assert!(is_known_command("config", None, None));
        assert!(is_known_command("config", Some("interactive"), None));
        assert!(!is_known_command("entry", Some("text"), None));
    }

    #[test]
    fn unknown_variants_are_rejected() {
        assert!(!is_known_command("workspace", None, None));
        assert!(!is_known_command("timer", Some("pause"), None));
        assert!(!is_known_command("entry", Some("text"), Some("show")));
    }

    #[test]
    fn cli_spec_routable_command_paths_are_known_commands() {
        let spec = crate::cli_spec::cli_spec();

        for path in spec.command_paths() {
            if matches!(path.first().copied(), Some("help")) {
                continue;
            }

            let (resource, action, subaction) = router_parts(&path);
            assert!(
                is_known_command(resource, action, subaction),
                "cli_spec command path is not accepted by router: {}",
                path.join(" ")
            );
        }
    }

    #[test]
    fn completion_shell_paths_are_known_commands() {
        for shell in crate::cli_spec::COMPLETION_SHELLS {
            assert!(is_known_command("completion", Some(shell), None));
        }
    }

    #[test]
    fn invalid_completion_shell_path_is_rejected() {
        assert!(!is_known_command("completion", None, None));
        assert!(!is_known_command("completion", Some("powershell"), None));
        assert!(!is_known_command("completion", Some("bash"), Some("extra")));
    }
}

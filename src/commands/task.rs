use crate::args::ParsedArgs;
use crate::client::{ClockifyClient, HttpTransport};
use crate::commands::list_columns::{
    format_tsv_rows, parse_optional_columns, validate_columns_with_format,
};
use crate::config;
use crate::error::CfdError;
use crate::format::{
    format_json, format_resource_id, format_text_blocks, format_text_fields, OutputFormat,
    OutputOptions, TextField,
};
use crate::types::{StoredConfig, Task};

pub fn execute<T: HttpTransport>(
    client: &ClockifyClient<T>,
    args: &ParsedArgs,
    workspace_id: &str,
    config_state: &StoredConfig,
) -> Result<(), CfdError> {
    match args.action.as_deref() {
        Some("list") => {
            validate_columns_with_format(args)?;
            let columns = parse_task_columns(args.flags.get("columns").map(String::as_str))?;
            let explicit_project = args.flags.get("project").map(String::as_str);
            let project_id = config::resolve_project(explicit_project, config_state)?;
            let tasks = client.list_tasks(workspace_id, &project_id)?;
            print_tasks(&tasks, &args.output, &columns)
        }
        Some("get") => {
            let project_id = args
                .positional
                .first()
                .ok_or_else(|| CfdError::message("usage: cfd task get <project-id> <task-id>"))?;
            let task_id = args
                .positional
                .get(1)
                .ok_or_else(|| CfdError::message("usage: cfd task get <project-id> <task-id>"))?;
            let task = client.get_task(workspace_id, project_id, task_id)?;
            print_task(&task, &args.output)
        }
        Some("create") => create_task(client, args, workspace_id),
        _ => Err(CfdError::message("usage: cfd task <list|get|create>")),
    }
}

fn create_task<T: HttpTransport>(
    client: &ClockifyClient<T>,
    args: &ParsedArgs,
    workspace_id: &str,
) -> Result<(), CfdError> {
    let project_id = args
        .flags
        .get("project")
        .map(String::as_str)
        .ok_or_else(|| CfdError::message("usage: cfd task create --project <id> --name <text>"))?;
    let name = args
        .flags
        .get("name")
        .map(String::as_str)
        .filter(|value| !value.trim().is_empty())
        .ok_or_else(|| CfdError::message("usage: cfd task create --project <id> --name <text>"))?;

    let task = client.create_task(workspace_id, project_id, name)?;
    println!("{}", format_resource_id(&task.id));
    Ok(())
}

fn print_tasks(
    tasks: &[Task],
    output: &OutputOptions,
    columns: &[TaskColumn],
) -> Result<(), CfdError> {
    match output.format {
        OutputFormat::Json => println!("{}", format_json(tasks)?),
        OutputFormat::Text => {
            if columns.is_empty() {
                println!(
                    "{}",
                    format_text_blocks(
                        &tasks
                            .iter()
                            .map(|task| format_task_text(task, output))
                            .collect::<Vec<_>>()
                    )
                );
            } else {
                println!("{}", format_task_table(tasks, columns));
            }
        }
    }
    Ok(())
}

fn print_task(task: &Task, output: &OutputOptions) -> Result<(), CfdError> {
    match output.format {
        OutputFormat::Json => println!("{}", format_json(task)?),
        OutputFormat::Text => println!("{}", format_task_text(task, output)),
    }
    Ok(())
}

fn format_task_text(task: &Task, output: &OutputOptions) -> String {
    format_text_fields(
        &[
            TextField {
                label: "id",
                value: &task.id,
                is_meta: true,
            },
            TextField {
                label: "name",
                value: &task.name,
                is_meta: false,
            },
        ],
        output,
    )
}

fn format_task_table(tasks: &[Task], columns: &[TaskColumn]) -> String {
    format_tsv_rows(
        &tasks
            .iter()
            .map(|task| {
                columns
                    .iter()
                    .map(|column| column.value(task))
                    .collect::<Vec<_>>()
            })
            .collect::<Vec<_>>(),
    )
}

fn parse_task_columns(value: Option<&str>) -> Result<Vec<TaskColumn>, CfdError> {
    parse_optional_columns(
        value,
        "usage: cfd task list --columns <id,name,project,...>",
        |item| match item {
            "id" => Ok(TaskColumn::Id),
            "name" => Ok(TaskColumn::Name),
            "project" => Ok(TaskColumn::Project),
            other => Err(CfdError::message(format!("invalid task column: {other}"))),
        },
    )
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TaskColumn {
    Id,
    Name,
    Project,
}

impl TaskColumn {
    fn value(self, task: &Task) -> String {
        match self {
            TaskColumn::Id => task.id.clone(),
            TaskColumn::Name => task.name.clone(),
            TaskColumn::Project => task.project_id.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use std::cell::RefCell;

    use super::*;
    use crate::error::CfdError;

    struct MockTransport {
        response: String,
        posted_body: RefCell<Option<String>>,
    }

    impl MockTransport {
        fn new(response: &str) -> Self {
            Self {
                response: response.to_owned(),
                posted_body: RefCell::new(None),
            }
        }
    }

    impl HttpTransport for MockTransport {
        fn get(&self, _url: &str, _api_key: &str) -> Result<String, CfdError> {
            Ok(self.response.clone())
        }

        fn post(&self, _url: &str, _api_key: &str, body: &str) -> Result<String, CfdError> {
            self.posted_body.replace(Some(body.to_owned()));
            Ok(self.response.clone())
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
    fn task_text_output_respects_no_meta() {
        let task = Task {
            id: "t1".into(),
            name: "ABC-1: Implement".into(),
            project_id: "p1".into(),
        };

        assert_eq!(
            format_task_text(&task, &OutputOptions::default()),
            "id: t1\nname: ABC-1: Implement"
        );
        assert_eq!(
            format_task_text(
                &task,
                &OutputOptions {
                    format: OutputFormat::Text,
                    no_meta: true,
                }
            ),
            "name: ABC-1: Implement"
        );
    }

    #[test]
    fn create_requires_project_and_name() {
        let client = ClockifyClient::new("secret".into(), MockTransport::new("{}"));
        let args = ParsedArgs {
            resource: Some("task".into()),
            action: Some("create".into()),
            subaction: None,
            positional: Vec::new(),
            flags: Default::default(),
            output: OutputOptions::default(),
            workspace: None,
            yes: false,
            no_rounding: false,
        };

        let error = execute(&client, &args, "w1", &StoredConfig::default())
            .unwrap_err()
            .to_string();

        assert!(error.contains("usage: cfd task create"));
    }

    #[test]
    fn task_columns_parse_and_render() {
        let columns = parse_task_columns(Some("name,project")).unwrap();
        let rendered = format_task_table(
            &[Task {
                id: "t1".into(),
                name: "ABC-1".into(),
                project_id: "p1".into(),
            }],
            &columns,
        );

        assert_eq!(rendered, "ABC-1\tp1");
    }

    #[test]
    fn task_columns_reject_invalid_name() {
        let error = parse_task_columns(Some("bogus")).unwrap_err().to_string();

        assert!(error.contains("invalid task column: bogus"));
    }
}

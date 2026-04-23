use crate::args::ParsedArgs;
use crate::client::{ClockifyClient, HttpTransport};
use crate::commands::list_columns::{
    format_tsv_rows, parse_optional_columns, validate_columns_with_format,
};
use crate::error::CfdError;
use crate::format::{
    format_json, format_text_blocks, format_text_fields, OutputFormat, OutputOptions, TextField,
};
use crate::types::Workspace;

pub fn execute<T: HttpTransport>(
    client: &ClockifyClient<T>,
    args: &ParsedArgs,
) -> Result<(), CfdError> {
    match args.action.as_deref() {
        Some("list") => {
            validate_columns_with_format(args)?;
            list_workspaces(client, args)
        }
        Some("get") => {
            let workspace_id = args
                .positional
                .first()
                .ok_or_else(|| CfdError::message("usage: cfd workspace get <id>"))?;
            get_workspace(client, workspace_id, &args.output)
        }
        _ => Err(CfdError::message("usage: cfd workspace <list|get>")),
    }
}

fn list_workspaces<T: HttpTransport>(
    client: &ClockifyClient<T>,
    args: &ParsedArgs,
) -> Result<(), CfdError> {
    let columns = parse_workspace_columns(args.flags.get("columns").map(String::as_str))?;
    let workspaces = client.list_workspaces()?;

    match args.output.format {
        OutputFormat::Json => println!("{}", format_json(&workspaces)?),
        OutputFormat::Text => {
            if columns.is_empty() {
                println!(
                    "{}",
                    format_text_blocks(
                        &workspaces
                            .iter()
                            .map(|workspace| format_workspace_text(workspace, &args.output))
                            .collect::<Vec<_>>()
                    )
                );
            } else {
                println!("{}", format_workspace_table(&workspaces, &columns));
            }
        }
    }

    Ok(())
}

fn get_workspace<T: HttpTransport>(
    client: &ClockifyClient<T>,
    workspace_id: &str,
    output: &OutputOptions,
) -> Result<(), CfdError> {
    let workspace = client.get_workspace(workspace_id)?;

    match output.format {
        OutputFormat::Json => println!("{}", format_json(&workspace)?),
        OutputFormat::Text => println!("{}", format_workspace_text(&workspace, output)),
    }

    Ok(())
}

fn format_workspace_text(workspace: &Workspace, output: &OutputOptions) -> String {
    format_text_fields(
        &[
            TextField {
                label: "id",
                value: &workspace.id,
                is_meta: true,
            },
            TextField {
                label: "name",
                value: &workspace.name,
                is_meta: false,
            },
        ],
        output,
    )
}

fn format_workspace_table(workspaces: &[Workspace], columns: &[WorkspaceColumn]) -> String {
    format_tsv_rows(
        &workspaces
            .iter()
            .map(|workspace| {
                columns
                    .iter()
                    .map(|column| column.value(workspace))
                    .collect::<Vec<_>>()
            })
            .collect::<Vec<_>>(),
    )
}

fn parse_workspace_columns(value: Option<&str>) -> Result<Vec<WorkspaceColumn>, CfdError> {
    parse_optional_columns(
        value,
        "usage: cfd workspace list --columns <id,name,...>",
        |item| match item {
            "id" => Ok(WorkspaceColumn::Id),
            "name" => Ok(WorkspaceColumn::Name),
            other => Err(CfdError::message(format!(
                "invalid workspace column: {other}"
            ))),
        },
    )
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum WorkspaceColumn {
    Id,
    Name,
}

impl WorkspaceColumn {
    fn value(self, workspace: &Workspace) -> String {
        match self {
            WorkspaceColumn::Id => workspace.id.clone(),
            WorkspaceColumn::Name => workspace.name.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use std::cell::RefCell;
    use std::rc::Rc;

    use super::*;

    struct MockTransport {
        body: String,
        requests: Rc<RefCell<Vec<String>>>,
    }

    impl MockTransport {
        fn new(body: &str) -> (Self, Rc<RefCell<Vec<String>>>) {
            let requests = Rc::new(RefCell::new(Vec::new()));
            (
                Self {
                    body: body.to_owned(),
                    requests: Rc::clone(&requests),
                },
                requests,
            )
        }
    }

    impl HttpTransport for MockTransport {
        fn get(&self, url: &str, _api_key: &str) -> Result<String, CfdError> {
            self.requests.borrow_mut().push(url.to_owned());
            Ok(self.body.clone())
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
    fn workspace_text_output_respects_no_meta() {
        let workspace = Workspace {
            id: "w1".into(),
            name: "Engineering".into(),
        };

        assert_eq!(
            format_workspace_text(
                &workspace,
                &OutputOptions {
                    format: OutputFormat::Text,
                    no_meta: false,
                }
            ),
            "id: w1\nname: Engineering"
        );
        assert_eq!(
            format_workspace_text(
                &workspace,
                &OutputOptions {
                    format: OutputFormat::Text,
                    no_meta: true,
                }
            ),
            "name: Engineering"
        );
    }

    #[test]
    fn workspace_columns_parse_and_render() {
        let columns = parse_workspace_columns(Some("name,id")).unwrap();
        let rendered = format_workspace_table(
            &[Workspace {
                id: "w1".into(),
                name: "Engineering".into(),
            }],
            &columns,
        );

        assert_eq!(rendered, "Engineering\tw1");
    }

    #[test]
    fn workspace_columns_require_value() {
        let error = parse_workspace_columns(Some("true"))
            .unwrap_err()
            .to_string();

        assert!(error.contains("usage: cfd workspace list --columns <id,name,...>"));
    }

    #[test]
    fn execute_list_hits_workspace_collection_endpoint() {
        let (transport, requests) = MockTransport::new(r#"[{"id":"w1","name":"Engineering"}]"#);
        let client = ClockifyClient::new("secret".into(), transport);
        let args = ParsedArgs {
            resource: Some("workspace".into()),
            action: Some("list".into()),
            subaction: None,
            positional: Vec::new(),
            flags: Default::default(),
            output: OutputOptions::default(),
            workspace: None,
            yes: false,
            no_rounding: false,
        };

        execute(&client, &args).unwrap();

        assert_eq!(
            requests.borrow().as_slice(),
            ["https://api.clockify.me/api/v1/workspaces"]
        );
    }

    #[test]
    fn execute_get_requires_id() {
        let (transport, _) = MockTransport::new("{}");
        let client = ClockifyClient::new("secret".into(), transport);
        let args = ParsedArgs {
            resource: Some("workspace".into()),
            action: Some("get".into()),
            subaction: None,
            positional: Vec::new(),
            flags: Default::default(),
            output: OutputOptions::default(),
            workspace: None,
            yes: false,
            no_rounding: false,
        };

        let error = execute(&client, &args).unwrap_err().to_string();

        assert!(error.contains("usage: cfd workspace get <id>"));
    }
}

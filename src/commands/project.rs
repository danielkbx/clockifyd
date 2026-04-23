use crate::args::ParsedArgs;
use crate::client::{ClockifyClient, HttpTransport};
use crate::commands::list_columns::{
    format_tsv_rows, parse_optional_columns, validate_columns_with_format,
};
use crate::error::CfdError;
use crate::format::{
    format_json, format_text_blocks, format_text_fields, OutputFormat, OutputOptions, TextField,
};
use crate::types::Project;
use std::collections::BTreeMap;

pub fn execute<T: HttpTransport>(
    client: &ClockifyClient<T>,
    args: &ParsedArgs,
    workspace_id: &str,
) -> Result<(), CfdError> {
    match args.action.as_deref() {
        Some("list") => {
            validate_columns_with_format(args)?;
            let columns = parse_project_columns(args.flags.get("columns").map(String::as_str))?;
            let projects = client.list_projects(workspace_id)?;
            let workspace_names = load_workspace_names_for_projects(client, &projects)?;
            print_projects(&projects, &workspace_names, &args.output, &columns)
        }
        Some("get") => {
            let project_id = args
                .positional
                .first()
                .ok_or_else(|| CfdError::message("usage: cfd project get <id>"))?;
            let project = client.get_project(workspace_id, project_id)?;
            let workspace_names = load_workspace_names_for_projects(client, &[project.clone()])?;
            print_project(&project, &workspace_names, &args.output)
        }
        _ => Err(CfdError::message("usage: cfd project <list|get>")),
    }
}

fn print_projects(
    projects: &[Project],
    workspace_names: &BTreeMap<String, String>,
    output: &OutputOptions,
    columns: &[ProjectColumn],
) -> Result<(), CfdError> {
    match output.format {
        OutputFormat::Json => println!("{}", format_json(projects)?),
        OutputFormat::Text => {
            if columns.is_empty() {
                println!(
                    "{}",
                    format_text_blocks(
                        &projects
                            .iter()
                            .map(|project| {
                                format_project_text(
                                    project,
                                    workspace_names.get(project.workspace_id.as_deref().unwrap_or("")),
                                    output,
                                )
                            })
                            .collect::<Vec<_>>()
                    )
                );
            } else {
                println!("{}", format_project_table(projects, workspace_names, columns));
            }
        }
    }
    Ok(())
}

fn print_project(
    project: &Project,
    workspace_names: &BTreeMap<String, String>,
    output: &OutputOptions,
) -> Result<(), CfdError> {
    match output.format {
        OutputFormat::Json => println!("{}", format_json(project)?),
        OutputFormat::Text => println!(
            "{}",
            format_project_get_text(
                project,
                workspace_names.get(project.workspace_id.as_deref().unwrap_or("")),
                output
            )
        ),
    }
    Ok(())
}

fn format_project_text(
    project: &Project,
    workspace_name: Option<&String>,
    output: &OutputOptions,
) -> String {
    format_text_fields(
        &[
            TextField {
                label: "id",
                value: &project.id,
                is_meta: true,
            },
            TextField {
                label: "name",
                value: &project.name,
                is_meta: false,
            },
            TextField {
                label: "workspaceName",
                value: workspace_name.map(String::as_str).unwrap_or(""),
                is_meta: false,
            },
        ],
        output,
    )
}

fn format_project_get_text(
    project: &Project,
    workspace_name: Option<&String>,
    output: &OutputOptions,
) -> String {
    let mut fields = vec![
        TextField {
            label: "id",
            value: &project.id,
            is_meta: true,
        },
        TextField {
            label: "name",
            value: &project.name,
            is_meta: false,
        },
        TextField {
            label: "workspaceName",
            value: workspace_name.map(String::as_str).unwrap_or(""),
            is_meta: false,
        },
    ];

    if let Some(client_id) = project.client_id.as_deref() {
        fields.push(TextField {
            label: "clientId",
            value: client_id,
            is_meta: false,
        });
    }

    if let Some(workspace_id) = project.workspace_id.as_deref() {
        fields.push(TextField {
            label: "workspaceId",
            value: workspace_id,
            is_meta: true,
        });
    }

    format_text_fields(&fields, output)
}

fn format_project_table(
    projects: &[Project],
    workspace_names: &BTreeMap<String, String>,
    columns: &[ProjectColumn],
) -> String {
    format_tsv_rows(
        &projects
            .iter()
            .map(|project| {
                columns
                    .iter()
                    .map(|column| column.value(project, workspace_names))
                    .collect::<Vec<_>>()
            })
            .collect::<Vec<_>>(),
    )
}

fn parse_project_columns(value: Option<&str>) -> Result<Vec<ProjectColumn>, CfdError> {
    parse_optional_columns(
        value,
        "usage: cfd project list --columns <id,name,client,workspaceId,workspaceName,...>",
        |item| match item {
            "id" => Ok(ProjectColumn::Id),
            "name" => Ok(ProjectColumn::Name),
            "client" => Ok(ProjectColumn::Client),
            "workspace" | "workspaceId" => Ok(ProjectColumn::Workspace),
            "workspaceName" => Ok(ProjectColumn::WorkspaceName),
            other => Err(CfdError::message(format!(
                "invalid project column: {other}"
            ))),
        },
    )
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ProjectColumn {
    Id,
    Name,
    Client,
    Workspace,
    WorkspaceName,
}

impl ProjectColumn {
    fn value(self, project: &Project, workspace_names: &BTreeMap<String, String>) -> String {
        match self {
            ProjectColumn::Id => project.id.clone(),
            ProjectColumn::Name => project.name.clone(),
            ProjectColumn::Client => project.client_id.clone().unwrap_or_default(),
            ProjectColumn::Workspace => project.workspace_id.clone().unwrap_or_default(),
            ProjectColumn::WorkspaceName => project
                .workspace_id
                .as_deref()
                .and_then(|id| workspace_names.get(id))
                .cloned()
                .unwrap_or_default(),
        }
    }
}

fn load_workspace_names_for_projects<T: HttpTransport>(
    client: &ClockifyClient<T>,
    projects: &[Project],
) -> Result<BTreeMap<String, String>, CfdError> {
    let workspace_ids = projects
        .iter()
        .filter_map(|project| project.workspace_id.as_deref())
        .collect::<std::collections::BTreeSet<_>>();
    if workspace_ids.is_empty() {
        return Ok(BTreeMap::new());
    }

    let workspaces = client.list_workspaces()?;
    Ok(workspaces
        .into_iter()
        .filter(|workspace| workspace_ids.contains(workspace.id.as_str()))
        .map(|workspace| (workspace.id, workspace.name))
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn project_text_output_respects_no_meta() {
        let project = Project {
            id: "p1".into(),
            name: "Clockify CLI".into(),
            client_id: Some("c1".into()),
            workspace_id: Some("w1".into()),
        };
        let workspace_name = "Engineering".to_string();

        assert_eq!(
            format_project_text(&project, Some(&workspace_name), &OutputOptions::default()),
            "id: p1\nname: Clockify CLI\nworkspaceName: Engineering"
        );
        assert_eq!(
            format_project_text(
                &project,
                Some(&workspace_name),
                &OutputOptions {
                    format: OutputFormat::Text,
                    no_meta: true,
                }
            ),
            "name: Clockify CLI\nworkspaceName: Engineering"
        );
    }

    #[test]
    fn project_get_text_output_includes_details() {
        let project = Project {
            id: "p1".into(),
            name: "Clockify CLI".into(),
            client_id: Some("c1".into()),
            workspace_id: Some("w1".into()),
        };
        let workspace_name = "Engineering".to_string();

        assert_eq!(
            format_project_get_text(&project, Some(&workspace_name), &OutputOptions::default()),
            "id: p1\nname: Clockify CLI\nworkspaceName: Engineering\nclientId: c1\nworkspaceId: w1"
        );
        assert_eq!(
            format_project_get_text(
                &project,
                Some(&workspace_name),
                &OutputOptions {
                    format: OutputFormat::Text,
                    no_meta: true,
                }
            ),
            "name: Clockify CLI\nworkspaceName: Engineering\nclientId: c1"
        );
    }

    #[test]
    fn project_columns_parse_and_render_missing_optionals() {
        let columns =
            parse_project_columns(Some("id,client,workspaceId,workspaceName")).unwrap();
        let rendered = format_project_table(
            &[Project {
                id: "p1".into(),
                name: "Clockify CLI".into(),
                client_id: None,
                workspace_id: None,
            }],
            &BTreeMap::new(),
            &columns,
        );

        assert_eq!(rendered, "p1\t\t\t");
    }

    #[test]
    fn project_columns_reject_invalid_name() {
        let error = parse_project_columns(Some("bogus"))
            .unwrap_err()
            .to_string();

        assert!(error.contains("invalid project column: bogus"));
    }
}

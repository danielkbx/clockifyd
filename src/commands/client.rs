use crate::args::ParsedArgs;
use crate::client::{ClockifyClient, HttpTransport};
use crate::commands::list_columns::{
    format_tsv_rows, parse_optional_columns, validate_columns_with_format,
};
use crate::error::CfdError;
use crate::format::{
    format_json, format_text_blocks, format_text_fields, OutputFormat, OutputOptions, TextField,
};
use crate::types::Client;

pub fn execute<T: HttpTransport>(
    client: &ClockifyClient<T>,
    args: &ParsedArgs,
    workspace_id: &str,
) -> Result<(), CfdError> {
    match args.action.as_deref() {
        Some("list") => {
            validate_columns_with_format(args)?;
            let columns = parse_client_columns(args.flags.get("columns").map(String::as_str))?;
            let clients = client.list_clients(workspace_id)?;
            print_clients(&clients, &args.output, &columns)
        }
        Some("get") => {
            let client_id = args
                .positional
                .first()
                .ok_or_else(|| CfdError::message("usage: cfd client get <id>"))?;
            let client_resource = client.get_client(workspace_id, client_id)?;
            print_client(&client_resource, &args.output)
        }
        _ => Err(CfdError::message("usage: cfd client <list|get>")),
    }
}

fn print_clients(
    clients: &[Client],
    output: &OutputOptions,
    columns: &[ClientColumn],
) -> Result<(), CfdError> {
    match output.format {
        OutputFormat::Json => println!("{}", format_json(clients)?),
        OutputFormat::Text => {
            if columns.is_empty() {
                println!(
                    "{}",
                    format_text_blocks(
                        &clients
                            .iter()
                            .map(|client_resource| format_client_text(client_resource, output))
                            .collect::<Vec<_>>()
                    )
                );
            } else {
                println!("{}", format_client_table(clients, columns));
            }
        }
    }
    Ok(())
}

fn print_client(client_resource: &Client, output: &OutputOptions) -> Result<(), CfdError> {
    match output.format {
        OutputFormat::Json => println!("{}", format_json(client_resource)?),
        OutputFormat::Text => println!("{}", format_client_text(client_resource, output)),
    }
    Ok(())
}

fn format_client_text(client_resource: &Client, output: &OutputOptions) -> String {
    format_text_fields(
        &[
            TextField {
                label: "id",
                value: &client_resource.id,
                is_meta: true,
            },
            TextField {
                label: "name",
                value: &client_resource.name,
                is_meta: false,
            },
        ],
        output,
    )
}

fn format_client_table(clients: &[Client], columns: &[ClientColumn]) -> String {
    format_tsv_rows(
        &clients
            .iter()
            .map(|client_resource| {
                columns
                    .iter()
                    .map(|column| column.value(client_resource))
                    .collect::<Vec<_>>()
            })
            .collect::<Vec<_>>(),
    )
}

fn parse_client_columns(value: Option<&str>) -> Result<Vec<ClientColumn>, CfdError> {
    parse_optional_columns(
        value,
        "usage: cfd client list --columns <id,name,...>",
        |item| match item {
            "id" => Ok(ClientColumn::Id),
            "name" => Ok(ClientColumn::Name),
            other => Err(CfdError::message(format!("invalid client column: {other}"))),
        },
    )
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ClientColumn {
    Id,
    Name,
}

impl ClientColumn {
    fn value(self, client_resource: &Client) -> String {
        match self {
            ClientColumn::Id => client_resource.id.clone(),
            ClientColumn::Name => client_resource.name.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn client_columns_parse_and_render() {
        let columns = parse_client_columns(Some("name,id")).unwrap();
        let rendered = format_client_table(
            &[Client {
                id: "c1".into(),
                name: "Acme".into(),
            }],
            &columns,
        );

        assert_eq!(rendered, "Acme\tc1");
    }

    #[test]
    fn client_columns_require_value() {
        let error = parse_client_columns(Some("true")).unwrap_err().to_string();

        assert!(error.contains("usage: cfd client list --columns <id,name,...>"));
    }
}

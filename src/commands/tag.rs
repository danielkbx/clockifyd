use crate::args::ParsedArgs;
use crate::client::{ClockifyClient, HttpTransport};
use crate::commands::list_columns::{
    format_tsv_rows, parse_optional_columns, validate_columns_with_format,
};
use crate::error::CfdError;
use crate::format::{
    format_json, format_text_blocks, format_text_fields, OutputFormat, OutputOptions, TextField,
};
use crate::types::Tag;

pub fn execute<T: HttpTransport>(
    client: &ClockifyClient<T>,
    args: &ParsedArgs,
    workspace_id: &str,
) -> Result<(), CfdError> {
    match args.action.as_deref() {
        Some("list") => {
            validate_columns_with_format(args)?;
            let columns = parse_tag_columns(args.flags.get("columns").map(String::as_str))?;
            let tags = client.list_tags(workspace_id)?;
            print_tags(&tags, &args.output, &columns)
        }
        Some("get") => {
            let tag_id = args
                .positional
                .first()
                .ok_or_else(|| CfdError::message("usage: cfd tag get <id>"))?;
            let tag = client.get_tag(workspace_id, tag_id)?;
            print_tag(&tag, &args.output)
        }
        _ => Err(CfdError::message("usage: cfd tag <list|get>")),
    }
}

fn print_tags(tags: &[Tag], output: &OutputOptions, columns: &[TagColumn]) -> Result<(), CfdError> {
    match output.format {
        OutputFormat::Json => println!("{}", format_json(tags)?),
        OutputFormat::Text => {
            if columns.is_empty() {
                println!(
                    "{}",
                    format_text_blocks(
                        &tags
                            .iter()
                            .map(|tag| format_tag_text(tag, output))
                            .collect::<Vec<_>>()
                    )
                );
            } else {
                println!("{}", format_tag_table(tags, columns));
            }
        }
    }
    Ok(())
}

fn print_tag(tag: &Tag, output: &OutputOptions) -> Result<(), CfdError> {
    match output.format {
        OutputFormat::Json => println!("{}", format_json(tag)?),
        OutputFormat::Text => println!("{}", format_tag_text(tag, output)),
    }
    Ok(())
}

fn format_tag_text(tag: &Tag, output: &OutputOptions) -> String {
    format_text_fields(
        &[
            TextField {
                label: "id",
                value: &tag.id,
                is_meta: true,
            },
            TextField {
                label: "name",
                value: &tag.name,
                is_meta: false,
            },
        ],
        output,
    )
}

fn format_tag_table(tags: &[Tag], columns: &[TagColumn]) -> String {
    format_tsv_rows(
        &tags
            .iter()
            .map(|tag| {
                columns
                    .iter()
                    .map(|column| column.value(tag))
                    .collect::<Vec<_>>()
            })
            .collect::<Vec<_>>(),
    )
}

fn parse_tag_columns(value: Option<&str>) -> Result<Vec<TagColumn>, CfdError> {
    parse_optional_columns(
        value,
        "usage: cfd tag list --columns <id,name,...>",
        |item| match item {
            "id" => Ok(TagColumn::Id),
            "name" => Ok(TagColumn::Name),
            other => Err(CfdError::message(format!("invalid tag column: {other}"))),
        },
    )
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TagColumn {
    Id,
    Name,
}

impl TagColumn {
    fn value(self, tag: &Tag) -> String {
        match self {
            TagColumn::Id => tag.id.clone(),
            TagColumn::Name => tag.name.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tag_columns_parse_and_render() {
        let columns = parse_tag_columns(Some("id,name")).unwrap();
        let rendered = format_tag_table(
            &[Tag {
                id: "tag1".into(),
                name: "billable".into(),
            }],
            &columns,
        );

        assert_eq!(rendered, "tag1\tbillable");
    }

    #[test]
    fn tag_columns_require_value() {
        let error = parse_tag_columns(Some("true")).unwrap_err().to_string();

        assert!(error.contains("usage: cfd tag list --columns <id,name,...>"));
    }
}

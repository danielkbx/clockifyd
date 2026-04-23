use crate::args::ParsedArgs;
use crate::error::CfdError;

pub fn validate_columns_with_format(args: &ParsedArgs) -> Result<(), CfdError> {
    if args.flags.contains_key("columns") && args.flags.contains_key("format") {
        return Err(CfdError::message(
            "use either --columns <list> or --format <text|json>, not both",
        ));
    }

    Ok(())
}

pub fn parse_optional_columns<T, F>(
    value: Option<&str>,
    usage: &str,
    mut parse_item: F,
) -> Result<Vec<T>, CfdError>
where
    F: FnMut(&str) -> Result<T, CfdError>,
{
    let Some(value) = value else {
        return Ok(Vec::new());
    };
    let value = value.trim();

    if value.is_empty() || value == "true" {
        return Err(CfdError::message(usage));
    }

    value
        .split(',')
        .map(|item| parse_item(item.trim()))
        .collect()
}

pub fn format_tsv_rows(rows: &[Vec<String>]) -> String {
    rows.iter()
        .map(|row| row.join("\t"))
        .collect::<Vec<_>>()
        .join("\n")
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use super::*;
    use crate::format::{OutputFormat, OutputOptions};

    fn parsed_args(flags: &[(&str, &str)]) -> ParsedArgs {
        ParsedArgs {
            resource: Some("project".into()),
            action: Some("list".into()),
            subaction: None,
            positional: Vec::new(),
            flags: flags
                .iter()
                .map(|(key, value)| (key.to_string(), value.to_string()))
                .collect::<HashMap<_, _>>(),
            output: OutputOptions {
                format: if flags.iter().any(|(key, _)| *key == "format") {
                    OutputFormat::Json
                } else {
                    OutputFormat::Text
                },
                no_meta: false,
            },
            workspace: None,
            yes: false,
            no_rounding: false,
        }
    }

    #[test]
    fn parse_optional_columns_requires_value() {
        let error = parse_optional_columns::<(), _>(
            Some("true"),
            "usage: cfd project list --columns <id,name,...>",
            |_| unreachable!(),
        )
        .unwrap_err()
        .to_string();

        assert!(error.contains("usage: cfd project list --columns <id,name,...>"));
    }

    #[test]
    fn parse_optional_columns_preserves_order() {
        let columns =
            parse_optional_columns(Some("name,id"), "usage", |item| -> Result<_, CfdError> {
                Ok(item.to_string())
            })
            .unwrap();

        assert_eq!(columns, vec!["name".to_string(), "id".to_string()]);
    }

    #[test]
    fn validate_columns_rejects_format_combo() {
        let error = validate_columns_with_format(&parsed_args(&[
            ("columns", "id,name"),
            ("format", "json"),
        ]))
        .unwrap_err()
        .to_string();

        assert!(error.contains("use either --columns <list> or --format"));
    }

    #[test]
    fn format_tsv_rows_joins_columns_and_rows() {
        let rendered = format_tsv_rows(&[
            vec!["w1".into(), "Engineering".into()],
            vec!["w2".into(), "Product".into()],
        ]);

        assert_eq!(rendered, "w1\tEngineering\nw2\tProduct");
    }
}

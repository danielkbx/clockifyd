use std::collections::HashMap;

use crate::format::{OutputFormat, OutputOptions};

pub(crate) const VALUE_FLAGS: &[&str] = &[
    "format",
    "workspace",
    "columns",
    "project",
    "start",
    "end",
    "text",
    "task",
    "tag",
    "duration",
    "description",
    "name",
    "scope",
];

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParsedArgs {
    pub resource: Option<String>,
    pub action: Option<String>,
    pub subaction: Option<String>,
    pub positional: Vec<String>,
    pub flags: HashMap<String, String>,
    pub output: OutputOptions,
    pub workspace: Option<String>,
    pub yes: bool,
    pub no_rounding: bool,
}

pub fn parse_args(argv: &[String]) -> ParsedArgs {
    let mut positional = Vec::new();
    let mut flags = HashMap::new();
    let mut i = 0;

    while i < argv.len() {
        let arg = &argv[i];
        if let Some(key) = arg.strip_prefix("--") {
            if let Some((name, value)) = key.split_once('=') {
                flags.insert(name.to_string(), value.to_string());
            } else if takes_value(key) && i + 1 < argv.len() && !argv[i + 1].starts_with('-') {
                flags.insert(key.to_string(), argv[i + 1].clone());
                i += 1;
            } else {
                flags.insert(key.to_string(), "true".to_string());
            }
        } else if arg == "-y" {
            flags.insert("y".to_string(), "true".to_string());
        } else {
            positional.push(arg.clone());
        }
        i += 1;
    }

    let resource = positional.first().cloned();
    let action = positional.get(1).cloned();
    let subaction = positional.get(2).cloned().filter(|_| {
        matches!(
            (resource.as_deref(), action.as_deref()),
            (Some("entry"), Some("text"))
        )
    });
    let consumed = 1 + usize::from(action.is_some()) + usize::from(subaction.is_some());
    let positional = if positional.len() > consumed {
        positional[consumed..].to_vec()
    } else {
        Vec::new()
    };

    ParsedArgs {
        resource,
        action,
        subaction,
        output: output_options(&flags),
        workspace: flags.get("workspace").cloned(),
        yes: flags.contains_key("y"),
        no_rounding: flags.contains_key("no-rounding"),
        positional,
        flags,
    }
}

pub(crate) fn takes_value(flag: &str) -> bool {
    VALUE_FLAGS.contains(&flag)
}

fn output_options(flags: &HashMap<String, String>) -> OutputOptions {
    let format = match flags.get("format").map(String::as_str) {
        Some("json" | "raw") => OutputFormat::Json,
        _ => OutputFormat::Text,
    };

    OutputOptions {
        format,
        no_meta: flags.contains_key("no-meta"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn args(input: &[&str]) -> ParsedArgs {
        parse_args(
            &input
                .iter()
                .map(|value| value.to_string())
                .collect::<Vec<_>>(),
        )
    }

    #[test]
    fn parses_resource_action_and_positionals() {
        let parsed = args(&["workspace", "get", "ws1"]);

        assert_eq!(parsed.resource.as_deref(), Some("workspace"));
        assert_eq!(parsed.action.as_deref(), Some("get"));
        assert_eq!(parsed.positional, vec!["ws1"]);
    }

    #[test]
    fn parses_entry_text_branch() {
        let parsed = args(&["entry", "text", "list", "--project", "p1"]);

        assert_eq!(parsed.resource.as_deref(), Some("entry"));
        assert_eq!(parsed.action.as_deref(), Some("text"));
        assert_eq!(parsed.subaction.as_deref(), Some("list"));
        assert_eq!(parsed.flags.get("project").map(String::as_str), Some("p1"));
    }

    #[test]
    fn parses_global_flags_centrally() {
        let parsed = args(&[
            "entry",
            "list",
            "--format",
            "raw",
            "--no-meta",
            "--workspace",
            "ws1",
            "--no-rounding",
            "-y",
        ]);

        assert_eq!(parsed.output.format, OutputFormat::Json);
        assert!(parsed.output.no_meta);
        assert_eq!(parsed.workspace.as_deref(), Some("ws1"));
        assert!(parsed.no_rounding);
        assert!(parsed.yes);
    }

    #[test]
    fn parses_equals_flags_and_text_filter() {
        let parsed = args(&[
            "entry",
            "list",
            "--text=focus",
            "--start=today",
            "--end=yesterday",
        ]);

        assert_eq!(parsed.flags.get("text").map(String::as_str), Some("focus"));
        assert_eq!(parsed.flags.get("start").map(String::as_str), Some("today"));
        assert_eq!(
            parsed.flags.get("end").map(String::as_str),
            Some("yesterday")
        );
    }

    #[test]
    fn keeps_flags_between_positionals() {
        let parsed = args(&["task", "--project", "p1", "create", "--name", "ABC-1"]);

        assert_eq!(parsed.resource.as_deref(), Some("task"));
        assert_eq!(parsed.action.as_deref(), Some("create"));
        assert_eq!(parsed.flags.get("project").map(String::as_str), Some("p1"));
        assert_eq!(parsed.flags.get("name").map(String::as_str), Some("ABC-1"));
    }

    #[test]
    fn bare_flags_without_value_become_true() {
        let parsed = args(&["project", "list", "--no-meta"]);

        assert_eq!(
            parsed.flags.get("no-meta").map(String::as_str),
            Some("true")
        );
    }

    #[test]
    fn parser_value_flags_are_represented_in_cli_spec() {
        let spec_options = crate::cli_spec::cli_spec().option_long_names();

        for flag in VALUE_FLAGS {
            assert!(
                takes_value(flag),
                "VALUE_FLAGS should stay aligned with takes_value: {flag}"
            );
            assert!(
                spec_options.contains(flag),
                "value-taking parser flag is missing from cli_spec: {flag}"
            );
        }
    }
}

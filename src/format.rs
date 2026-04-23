#![allow(dead_code)]

use serde::Serialize;

use crate::error::CfdError;
use crate::types::EntryTextItem;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, serde::Deserialize, Default)]
pub enum OutputFormat {
    #[default]
    #[serde(rename = "text")]
    Text,
    #[serde(rename = "json")]
    Json,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, serde::Deserialize, Default)]
pub struct OutputOptions {
    pub format: OutputFormat,
    #[serde(default)]
    pub no_meta: bool,
}

pub struct TextField<'a> {
    pub label: &'a str,
    pub value: &'a str,
    pub is_meta: bool,
}

pub fn format_json<T: Serialize + ?Sized>(value: &T) -> Result<String, CfdError> {
    serde_json::to_string_pretty(value).map_err(Into::into)
}

pub fn format_text_fields(fields: &[TextField<'_>], options: &OutputOptions) -> String {
    fields
        .iter()
        .filter(|field| !(options.no_meta && field.is_meta))
        .map(|field| format!("{}: {}", field.label, field.value))
        .collect::<Vec<_>>()
        .join("\n")
}

pub fn format_text_blocks(blocks: &[String]) -> String {
    blocks.join("\n\n")
}

pub fn format_resource_id(id: &str) -> String {
    id.to_owned()
}

pub fn format_entry_text_items(
    items: &[EntryTextItem],
    options: &OutputOptions,
) -> Result<String, CfdError> {
    match options.format {
        OutputFormat::Json => format_json(items),
        OutputFormat::Text => {
            let blocks = items
                .iter()
                .map(|item| {
                    if options.no_meta {
                        item.text.clone()
                    } else {
                        match item.usage_count {
                            Some(count) => format!(
                                "text: {}\nlastUsed: {}\ncount: {}",
                                item.text, item.last_used, count
                            ),
                            None => format!("text: {}\nlastUsed: {}", item.text, item.last_used),
                        }
                    }
                })
                .collect::<Vec<_>>();
            Ok(format_text_blocks(&blocks))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::User;

    #[test]
    fn formats_text_and_json_objects() {
        let user = User {
            id: "u1".into(),
            name: "Ada".into(),
            email: "ada@example.com".into(),
            active_workspace: None,
            default_workspace: None,
        };
        let text = format_text_fields(
            &[
                TextField {
                    label: "id",
                    value: &user.id,
                    is_meta: true,
                },
                TextField {
                    label: "name",
                    value: &user.name,
                    is_meta: false,
                },
                TextField {
                    label: "email",
                    value: &user.email,
                    is_meta: false,
                },
            ],
            &OutputOptions::default(),
        );
        let json = format_json(&user).unwrap();

        assert_eq!(text, "id: u1\nname: Ada\nemail: ada@example.com");
        assert!(json.contains("\"email\": \"ada@example.com\""));
    }

    #[test]
    fn no_meta_removes_meta_fields() {
        let text = format_text_fields(
            &[
                TextField {
                    label: "id",
                    value: "u1",
                    is_meta: true,
                },
                TextField {
                    label: "name",
                    value: "Ada",
                    is_meta: false,
                },
            ],
            &OutputOptions {
                format: OutputFormat::Text,
                no_meta: true,
            },
        );

        assert_eq!(text, "name: Ada");
    }

    #[test]
    fn create_update_output_is_stdout_id_only() {
        assert_eq!(format_resource_id("abc123"), "abc123");
    }

    #[test]
    fn entry_text_list_is_line_based_without_meta() {
        let rendered = format_entry_text_items(
            &[
                EntryTextItem {
                    text: "ABC-1".into(),
                    last_used: "2026-04-23T09:00:00Z".into(),
                    usage_count: Some(4),
                },
                EntryTextItem {
                    text: "ABC-2".into(),
                    last_used: "2026-04-23T10:00:00Z".into(),
                    usage_count: Some(2),
                },
            ],
            &OutputOptions {
                format: OutputFormat::Text,
                no_meta: true,
            },
        )
        .unwrap();

        assert_eq!(rendered, "ABC-1\n\nABC-2");
    }
}

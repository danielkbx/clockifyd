#![allow(dead_code)]

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct StoredConfig {
    #[serde(rename = "apiKey", skip_serializing_if = "Option::is_none")]
    pub api_key: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub workspace: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rounding: Option<RoundingMode>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub project: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum RoundingMode {
    #[default]
    #[serde(rename = "off")]
    Off,
    #[serde(rename = "1m")]
    OneMinute,
    #[serde(rename = "5m")]
    FiveMinutes,
    #[serde(rename = "10m")]
    TenMinutes,
    #[serde(rename = "15m")]
    FifteenMinutes,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct User {
    pub id: String,
    pub name: String,
    pub email: String,
    #[serde(rename = "activeWorkspace", skip_serializing_if = "Option::is_none")]
    pub active_workspace: Option<String>,
    #[serde(rename = "defaultWorkspace", skip_serializing_if = "Option::is_none")]
    pub default_workspace: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Workspace {
    pub id: String,
    pub name: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Project {
    pub id: String,
    pub name: String,
    #[serde(rename = "clientId", skip_serializing_if = "Option::is_none")]
    pub client_id: Option<String>,
    #[serde(rename = "workspaceId", skip_serializing_if = "Option::is_none")]
    pub workspace_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Client {
    pub id: String,
    pub name: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Tag {
    pub id: String,
    pub name: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Task {
    pub id: String,
    pub name: String,
    #[serde(rename = "projectId")]
    pub project_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TimeInterval {
    pub start: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub end: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub duration: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TimeEntry {
    pub id: String,
    #[serde(rename = "workspaceId")]
    pub workspace_id: String,
    #[serde(rename = "userId", skip_serializing_if = "Option::is_none")]
    pub user_id: Option<String>,
    #[serde(rename = "projectId", skip_serializing_if = "Option::is_none")]
    pub project_id: Option<String>,
    #[serde(rename = "taskId", skip_serializing_if = "Option::is_none")]
    pub task_id: Option<String>,
    #[serde(
        rename = "tagIds",
        default,
        deserialize_with = "deserialize_null_vec",
        skip_serializing_if = "Vec::is_empty"
    )]
    pub tag_ids: Vec<String>,
    #[serde(
        default,
        deserialize_with = "deserialize_null_string",
        skip_serializing_if = "String::is_empty"
    )]
    pub description: String,
    #[serde(rename = "timeInterval")]
    pub time_interval: TimeInterval,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EntryTextItem {
    pub text: String,
    #[serde(rename = "lastUsed")]
    pub last_used: String,
    #[serde(rename = "count", skip_serializing_if = "Option::is_none")]
    pub usage_count: Option<u64>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct EntryTextQuery {
    pub project: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub search: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub limit: Option<usize>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct EntryFilters {
    pub start: Option<String>,
    pub end: Option<String>,
    pub project: Option<String>,
    pub task: Option<String>,
    pub tags: Vec<String>,
    pub description: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OverlapWarning {
    pub overlapping_ids: Vec<String>,
}

fn deserialize_null_vec<'de, D, T>(deserializer: D) -> Result<Vec<T>, D::Error>
where
    D: serde::Deserializer<'de>,
    T: Deserialize<'de>,
{
    Ok(Option::<Vec<T>>::deserialize(deserializer)?.unwrap_or_default())
}

fn deserialize_null_string<'de, D>(deserializer: D) -> Result<String, D::Error>
where
    D: serde::Deserializer<'de>,
{
    Ok(Option::<String>::deserialize(deserializer)?.unwrap_or_default())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stored_config_serializes_expected_keys() {
        let config = StoredConfig {
            api_key: Some("secret".into()),
            workspace: Some("ws1".into()),
            rounding: Some(RoundingMode::FifteenMinutes),
            project: Some("pr1".into()),
        };

        let value = serde_json::to_value(config).unwrap();

        assert_eq!(value["apiKey"], "secret");
        assert_eq!(value["workspace"], "ws1");
        assert_eq!(value["rounding"], "15m");
        assert_eq!(value["project"], "pr1");
    }

    #[test]
    fn stored_config_deserializes_expected_keys() {
        let json = r#"{
            "apiKey": "secret",
            "workspace": "ws1",
            "rounding": "5m",
            "project": "pr1"
        }"#;

        let config: StoredConfig = serde_json::from_str(json).unwrap();

        assert_eq!(config.api_key.as_deref(), Some("secret"));
        assert_eq!(config.workspace.as_deref(), Some("ws1"));
        assert_eq!(config.rounding, Some(RoundingMode::FiveMinutes));
        assert_eq!(config.project.as_deref(), Some("pr1"));
    }

    #[test]
    fn rounding_mode_round_trips() {
        for (mode, expected) in [
            (RoundingMode::Off, "\"off\""),
            (RoundingMode::OneMinute, "\"1m\""),
            (RoundingMode::FiveMinutes, "\"5m\""),
            (RoundingMode::TenMinutes, "\"10m\""),
            (RoundingMode::FifteenMinutes, "\"15m\""),
        ] {
            let json = serde_json::to_string(&mode).unwrap();
            let parsed: RoundingMode = serde_json::from_str(&json).unwrap();

            assert_eq!(json, expected);
            assert_eq!(parsed, mode);
        }
    }

    #[test]
    fn base_api_types_deserialize() {
        let user: User = serde_json::from_str(
            r#"{
                "id": "u1",
                "name": "Ada Lovelace",
                "email": "ada@example.com",
                "activeWorkspace": "w1",
                "defaultWorkspace": "w0"
            }"#,
        )
        .unwrap();
        let workspace: Workspace =
            serde_json::from_str(r#"{ "id": "w1", "name": "Engineering" }"#).unwrap();
        let project: Project = serde_json::from_str(
            r#"{
                "id": "p1",
                "name": "Clockify CLI",
                "clientId": "c1",
                "workspaceId": "w1"
            }"#,
        )
        .unwrap();
        let entry: TimeEntry = serde_json::from_str(
            r#"{
                "id": "e1",
                "workspaceId": "w1",
                "userId": "u1",
                "projectId": "p1",
                "taskId": "t1",
                "tagIds": ["tag-1"],
                "description": "Build foundation",
                "timeInterval": {
                    "start": "2026-04-23T09:00:00Z",
                    "end": "2026-04-23T10:00:00Z",
                    "duration": "PT1H"
                }
            }"#,
        )
        .unwrap();

        assert_eq!(user.active_workspace.as_deref(), Some("w1"));
        assert_eq!(workspace.name, "Engineering");
        assert_eq!(project.client_id.as_deref(), Some("c1"));
        assert_eq!(entry.user_id.as_deref(), Some("u1"));
        assert_eq!(
            entry.time_interval.end.as_deref(),
            Some("2026-04-23T10:00:00Z")
        );
        assert_eq!(entry.tag_ids, vec!["tag-1"]);
    }

    #[test]
    fn time_entry_accepts_null_tag_ids() {
        let entry: TimeEntry = serde_json::from_str(
            r#"{
                "id": "e1",
                "workspaceId": "w1",
                "userId": "u1",
                "projectId": "p1",
                "taskId": null,
                "tagIds": null,
                "description": "Build foundation",
                "timeInterval": {
                    "start": "2026-04-23T09:00:00Z",
                    "end": "2026-04-23T10:00:00Z",
                    "duration": "PT1H"
                }
            }"#,
        )
        .unwrap();

        assert!(entry.tag_ids.is_empty());
    }

    #[test]
    fn time_entry_accepts_null_description() {
        let entry: TimeEntry = serde_json::from_str(
            r#"{
                "id": "e1",
                "workspaceId": "w1",
                "description": null,
                "timeInterval": {
                    "start": "2026-04-23T09:00:00Z"
                }
            }"#,
        )
        .unwrap();

        assert_eq!(entry.description, "");
    }
}

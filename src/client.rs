#![allow(dead_code)]

use crate::error::CfdError;
use crate::types::{Client, EntryFilters, Project, Tag, Task, TimeEntry, User, Workspace};

const BASE_URL: &str = "https://api.clockify.me/api/v1";

pub trait HttpTransport {
    fn get(&self, url: &str, api_key: &str) -> Result<String, CfdError>;
    fn post(&self, url: &str, api_key: &str, body: &str) -> Result<String, CfdError>;
    fn put(&self, url: &str, api_key: &str, body: &str) -> Result<String, CfdError>;
    fn patch(&self, url: &str, api_key: &str, body: &str) -> Result<String, CfdError>;
    fn delete(&self, url: &str, api_key: &str) -> Result<(), CfdError>;
}

pub struct UreqTransport;

impl HttpTransport for UreqTransport {
    fn get(&self, url: &str, api_key: &str) -> Result<String, CfdError> {
        let mut response = ureq::get(url)
            .header("X-Api-Key", api_key)
            .call()
            .map_err(|error| map_ureq_error(error, api_key))?;

        response
            .body_mut()
            .read_to_string()
            .map_err(|error| CfdError::transport(redact_secret(&error.to_string(), api_key)))
    }

    fn post(&self, url: &str, api_key: &str, body: &str) -> Result<String, CfdError> {
        let mut response = ureq::post(url)
            .header("X-Api-Key", api_key)
            .header("Content-Type", "application/json")
            .send(body)
            .map_err(|error| map_ureq_error(error, api_key))?;

        response
            .body_mut()
            .read_to_string()
            .map_err(|error| CfdError::transport(redact_secret(&error.to_string(), api_key)))
    }

    fn put(&self, url: &str, api_key: &str, body: &str) -> Result<String, CfdError> {
        let mut response = ureq::put(url)
            .header("X-Api-Key", api_key)
            .header("Content-Type", "application/json")
            .send(body)
            .map_err(|error| map_ureq_error(error, api_key))?;

        response
            .body_mut()
            .read_to_string()
            .map_err(|error| CfdError::transport(redact_secret(&error.to_string(), api_key)))
    }

    fn patch(&self, url: &str, api_key: &str, body: &str) -> Result<String, CfdError> {
        let mut response = ureq::patch(url)
            .header("X-Api-Key", api_key)
            .header("Content-Type", "application/json")
            .send(body)
            .map_err(|error| map_ureq_error(error, api_key))?;

        response
            .body_mut()
            .read_to_string()
            .map_err(|error| CfdError::transport(redact_secret(&error.to_string(), api_key)))
    }

    fn delete(&self, url: &str, api_key: &str) -> Result<(), CfdError> {
        ureq::delete(url)
            .header("X-Api-Key", api_key)
            .call()
            .map_err(|error| map_ureq_error(error, api_key))?;
        Ok(())
    }
}

pub struct ClockifyClient<T: HttpTransport> {
    api_key: String,
    base_url: String,
    transport: T,
}

impl<T: HttpTransport> ClockifyClient<T> {
    pub fn new(api_key: String, transport: T) -> Self {
        let base_url = std::env::var("CFD_BASE_URL")
            .ok()
            .map(|value| value.trim().to_owned())
            .filter(|value| !value.is_empty())
            .unwrap_or_else(|| BASE_URL.to_string());
        Self::with_base_url(api_key, base_url, transport)
    }

    fn with_base_url(api_key: String, base_url: String, transport: T) -> Self {
        Self {
            api_key,
            base_url,
            transport,
        }
    }

    pub fn get_current_user(&self) -> Result<User, CfdError> {
        self.get_json("/user")
    }

    pub fn list_workspaces(&self) -> Result<Vec<Workspace>, CfdError> {
        self.get_json("/workspaces")
    }

    pub fn get_workspace(&self, id: &str) -> Result<Workspace, CfdError> {
        self.get_json(&format!("/workspaces/{id}"))
    }

    pub fn list_projects(&self, workspace_id: &str) -> Result<Vec<Project>, CfdError> {
        self.get_json(&format!("/workspaces/{workspace_id}/projects"))
    }

    pub fn get_project(&self, workspace_id: &str, id: &str) -> Result<Project, CfdError> {
        self.get_json(&format!("/workspaces/{workspace_id}/projects/{id}"))
    }

    pub fn list_clients(&self, workspace_id: &str) -> Result<Vec<Client>, CfdError> {
        self.get_json(&format!("/workspaces/{workspace_id}/clients"))
    }

    pub fn get_client(&self, workspace_id: &str, id: &str) -> Result<Client, CfdError> {
        self.get_json(&format!("/workspaces/{workspace_id}/clients/{id}"))
    }

    pub fn list_tags(&self, workspace_id: &str) -> Result<Vec<Tag>, CfdError> {
        self.get_json(&format!("/workspaces/{workspace_id}/tags"))
    }

    pub fn get_tag(&self, workspace_id: &str, id: &str) -> Result<Tag, CfdError> {
        self.get_json(&format!("/workspaces/{workspace_id}/tags/{id}"))
    }

    pub fn list_tasks(&self, workspace_id: &str, project_id: &str) -> Result<Vec<Task>, CfdError> {
        self.get_json(&format!(
            "/workspaces/{workspace_id}/projects/{project_id}/tasks"
        ))
    }

    pub fn get_task(
        &self,
        workspace_id: &str,
        project_id: &str,
        task_id: &str,
    ) -> Result<Task, CfdError> {
        self.get_json(&format!(
            "/workspaces/{workspace_id}/projects/{project_id}/tasks/{task_id}"
        ))
    }

    pub fn create_task(
        &self,
        workspace_id: &str,
        project_id: &str,
        name: &str,
    ) -> Result<Task, CfdError> {
        let path = format!("/workspaces/{workspace_id}/projects/{project_id}/tasks");
        let body = serde_json::json!({ "name": name }).to_string();
        self.post_json(&path, &body)
    }

    pub fn list_time_entries(
        &self,
        workspace_id: &str,
        user_id: &str,
        filters: &EntryFilters,
    ) -> Result<Vec<TimeEntry>, CfdError> {
        let path = format!(
            "/workspaces/{workspace_id}/user/{user_id}/time-entries{}",
            entry_query_string(filters)
        );
        self.get_json(&path)
    }

    pub fn get_time_entry(&self, workspace_id: &str, id: &str) -> Result<TimeEntry, CfdError> {
        self.get_json(&format!("/workspaces/{workspace_id}/time-entries/{id}"))
    }

    pub fn create_time_entry(
        &self,
        workspace_id: &str,
        body: &serde_json::Value,
    ) -> Result<TimeEntry, CfdError> {
        self.post_json(
            &format!("/workspaces/{workspace_id}/time-entries"),
            &body.to_string(),
        )
    }

    pub fn update_time_entry(
        &self,
        workspace_id: &str,
        entry_id: &str,
        body: &serde_json::Value,
    ) -> Result<TimeEntry, CfdError> {
        self.put_json(
            &format!("/workspaces/{workspace_id}/time-entries/{entry_id}"),
            &body.to_string(),
        )
    }

    pub fn delete_time_entry(&self, workspace_id: &str, entry_id: &str) -> Result<(), CfdError> {
        self.delete_path(&format!(
            "/workspaces/{workspace_id}/time-entries/{entry_id}"
        ))
    }

    pub fn get_current_timers(&self, workspace_id: &str) -> Result<Vec<TimeEntry>, CfdError> {
        self.get_json(&format!(
            "/workspaces/{workspace_id}/time-entries/status/in-progress"
        ))
    }

    pub fn stop_timer(
        &self,
        workspace_id: &str,
        user_id: &str,
        end: &str,
    ) -> Result<TimeEntry, CfdError> {
        self.patch_json(
            &format!("/workspaces/{workspace_id}/user/{user_id}/time-entries"),
            &serde_json::json!({ "end": end }).to_string(),
        )
    }

    fn get_json<R>(&self, path: &str) -> Result<R, CfdError>
    where
        R: serde::de::DeserializeOwned,
    {
        let url = format!("{}{}", self.base_url, path);
        let body = self
            .transport
            .get(&url, &self.api_key)
            .map_err(|error| redact_error(error, &self.api_key))?;
        serde_json::from_str(&body).map_err(Into::into)
    }

    fn post_json<R>(&self, path: &str, body: &str) -> Result<R, CfdError>
    where
        R: serde::de::DeserializeOwned,
    {
        let url = format!("{}{}", self.base_url, path);
        let response_body = self
            .transport
            .post(&url, &self.api_key, body)
            .map_err(|error| redact_error(error, &self.api_key))?;
        serde_json::from_str(&response_body).map_err(Into::into)
    }

    fn put_json<R>(&self, path: &str, body: &str) -> Result<R, CfdError>
    where
        R: serde::de::DeserializeOwned,
    {
        let url = format!("{}{}", self.base_url, path);
        let response_body = self
            .transport
            .put(&url, &self.api_key, body)
            .map_err(|error| redact_error(error, &self.api_key))?;
        serde_json::from_str(&response_body).map_err(Into::into)
    }

    fn patch_json<R>(&self, path: &str, body: &str) -> Result<R, CfdError>
    where
        R: serde::de::DeserializeOwned,
    {
        let url = format!("{}{}", self.base_url, path);
        let response_body = self
            .transport
            .patch(&url, &self.api_key, body)
            .map_err(|error| redact_error(error, &self.api_key))?;
        serde_json::from_str(&response_body).map_err(Into::into)
    }

    fn delete_path(&self, path: &str) -> Result<(), CfdError> {
        let url = format!("{}{}", self.base_url, path);
        self.transport
            .delete(&url, &self.api_key)
            .map_err(|error| redact_error(error, &self.api_key))
    }
}

fn entry_query_string(filters: &EntryFilters) -> String {
    let mut pairs = Vec::new();

    if let Some(start) = &filters.start {
        pairs.push(format!("start={}", urlencoding::encode(start)));
    }
    if let Some(end) = &filters.end {
        pairs.push(format!("end={}", urlencoding::encode(end)));
    }
    if let Some(project) = &filters.project {
        pairs.push(format!("project={}", urlencoding::encode(project)));
    }
    if let Some(task) = &filters.task {
        pairs.push(format!("task={}", urlencoding::encode(task)));
    }
    if let Some(description) = &filters.description {
        pairs.push(format!("description={}", urlencoding::encode(description)));
    }
    for tag in &filters.tags {
        pairs.push(format!("tags={}", urlencoding::encode(tag)));
    }

    if pairs.is_empty() {
        String::new()
    } else {
        format!("?{}", pairs.join("&"))
    }
}

fn map_ureq_error(error: ureq::Error, api_key: &str) -> CfdError {
    match error {
        ureq::Error::StatusCode(status) => CfdError::HttpStatus { status },
        other => CfdError::transport(redact_secret(&other.to_string(), api_key)),
    }
}

fn redact_error(error: CfdError, api_key: &str) -> CfdError {
    match error {
        CfdError::Transport { message } => CfdError::transport(redact_secret(&message, api_key)),
        CfdError::Message(message) => CfdError::message(redact_secret(&message, api_key)),
        other => other,
    }
}

fn redact_secret(message: &str, secret: &str) -> String {
    if secret.is_empty() {
        return message.to_owned();
    }

    message.replace(secret, "[REDACTED]")
}

#[cfg(test)]
mod tests {
    use std::cell::RefCell;

    use super::*;

    #[derive(Debug, Clone)]
    struct RecordedRequest {
        method: String,
        url: String,
        api_key: String,
        body: Option<String>,
    }

    enum MockResponse {
        Success(String),
        Error(CfdError),
    }

    struct MockTransport {
        last_request: RefCell<Option<RecordedRequest>>,
        response: MockResponse,
    }

    impl MockTransport {
        fn success(body: &str) -> Self {
            Self {
                last_request: RefCell::new(None),
                response: MockResponse::Success(body.to_owned()),
            }
        }

        fn failure(error: CfdError) -> Self {
            Self {
                last_request: RefCell::new(None),
                response: MockResponse::Error(error),
            }
        }

        fn request(&self) -> RecordedRequest {
            self.last_request.borrow().clone().unwrap()
        }
    }

    impl HttpTransport for MockTransport {
        fn get(&self, url: &str, api_key: &str) -> Result<String, CfdError> {
            self.last_request.replace(Some(RecordedRequest {
                method: "GET".into(),
                url: url.to_owned(),
                api_key: api_key.to_owned(),
                body: None,
            }));

            match &self.response {
                MockResponse::Success(body) => Ok(body.clone()),
                MockResponse::Error(CfdError::HttpStatus { status }) => {
                    Err(CfdError::HttpStatus { status: *status })
                }
                MockResponse::Error(CfdError::Transport { message }) => {
                    Err(CfdError::transport(message.clone()))
                }
                MockResponse::Error(CfdError::Message(message)) => {
                    Err(CfdError::message(message.clone()))
                }
                MockResponse::Error(CfdError::Io(error)) => {
                    Err(CfdError::transport(error.to_string()))
                }
                MockResponse::Error(CfdError::Json(error)) => {
                    Err(CfdError::transport(error.to_string()))
                }
            }
        }

        fn post(&self, url: &str, api_key: &str, body: &str) -> Result<String, CfdError> {
            self.last_request.replace(Some(RecordedRequest {
                method: "POST".into(),
                url: url.to_owned(),
                api_key: api_key.to_owned(),
                body: Some(body.to_owned()),
            }));

            match &self.response {
                MockResponse::Success(body) => Ok(body.clone()),
                MockResponse::Error(CfdError::HttpStatus { status }) => {
                    Err(CfdError::HttpStatus { status: *status })
                }
                MockResponse::Error(CfdError::Transport { message }) => {
                    Err(CfdError::transport(message.clone()))
                }
                MockResponse::Error(CfdError::Message(message)) => {
                    Err(CfdError::message(message.clone()))
                }
                MockResponse::Error(CfdError::Io(error)) => {
                    Err(CfdError::transport(error.to_string()))
                }
                MockResponse::Error(CfdError::Json(error)) => {
                    Err(CfdError::transport(error.to_string()))
                }
            }
        }

        fn put(&self, url: &str, api_key: &str, body: &str) -> Result<String, CfdError> {
            self.last_request.replace(Some(RecordedRequest {
                method: "PUT".into(),
                url: url.to_owned(),
                api_key: api_key.to_owned(),
                body: Some(body.to_owned()),
            }));

            match &self.response {
                MockResponse::Success(body) => Ok(body.clone()),
                MockResponse::Error(CfdError::HttpStatus { status }) => {
                    Err(CfdError::HttpStatus { status: *status })
                }
                MockResponse::Error(CfdError::Transport { message }) => {
                    Err(CfdError::transport(message.clone()))
                }
                MockResponse::Error(CfdError::Message(message)) => {
                    Err(CfdError::message(message.clone()))
                }
                MockResponse::Error(CfdError::Io(error)) => {
                    Err(CfdError::transport(error.to_string()))
                }
                MockResponse::Error(CfdError::Json(error)) => {
                    Err(CfdError::transport(error.to_string()))
                }
            }
        }

        fn patch(&self, url: &str, api_key: &str, body: &str) -> Result<String, CfdError> {
            self.last_request.replace(Some(RecordedRequest {
                method: "PATCH".into(),
                url: url.to_owned(),
                api_key: api_key.to_owned(),
                body: Some(body.to_owned()),
            }));

            match &self.response {
                MockResponse::Success(body) => Ok(body.clone()),
                MockResponse::Error(CfdError::HttpStatus { status }) => {
                    Err(CfdError::HttpStatus { status: *status })
                }
                MockResponse::Error(CfdError::Transport { message }) => {
                    Err(CfdError::transport(message.clone()))
                }
                MockResponse::Error(CfdError::Message(message)) => {
                    Err(CfdError::message(message.clone()))
                }
                MockResponse::Error(CfdError::Io(error)) => {
                    Err(CfdError::transport(error.to_string()))
                }
                MockResponse::Error(CfdError::Json(error)) => {
                    Err(CfdError::transport(error.to_string()))
                }
            }
        }

        fn delete(&self, url: &str, api_key: &str) -> Result<(), CfdError> {
            self.last_request.replace(Some(RecordedRequest {
                method: "DELETE".into(),
                url: url.to_owned(),
                api_key: api_key.to_owned(),
                body: None,
            }));

            match &self.response {
                MockResponse::Success(_) => Ok(()),
                MockResponse::Error(CfdError::HttpStatus { status }) => {
                    Err(CfdError::HttpStatus { status: *status })
                }
                MockResponse::Error(CfdError::Transport { message }) => {
                    Err(CfdError::transport(message.clone()))
                }
                MockResponse::Error(CfdError::Message(message)) => {
                    Err(CfdError::message(message.clone()))
                }
                MockResponse::Error(CfdError::Io(error)) => {
                    Err(CfdError::transport(error.to_string()))
                }
                MockResponse::Error(CfdError::Json(error)) => {
                    Err(CfdError::transport(error.to_string()))
                }
            }
        }
    }

    #[test]
    fn current_user_uses_expected_url_and_header() {
        let transport =
            MockTransport::success(r#"{"id":"u1","name":"Ada","email":"ada@example.com"}"#);
        let client = ClockifyClient::with_base_url(
            "secret-key".into(),
            "https://example.test/api/v1".into(),
            transport,
        );

        let user = client.get_current_user().unwrap();
        let request = client.transport.request();

        assert_eq!(request.method, "GET");
        assert_eq!(request.url, "https://example.test/api/v1/user");
        assert_eq!(request.api_key, "secret-key");
        assert_eq!(user.email, "ada@example.com");
    }

    #[test]
    fn list_workspaces_parses_json() {
        let transport = MockTransport::success(
            r#"[{"id":"w1","name":"Engineering"},{"id":"w2","name":"Support"}]"#,
        );
        let client = ClockifyClient::new("secret-key".into(), transport);

        let workspaces = client.list_workspaces().unwrap();

        assert_eq!(workspaces.len(), 2);
        assert_eq!(workspaces[0].id, "w1");
        assert_eq!(workspaces[1].name, "Support");
    }

    #[test]
    fn get_workspace_uses_workspace_path() {
        let transport = MockTransport::success(r#"{"id":"w1","name":"Engineering"}"#);
        let client = ClockifyClient::with_base_url(
            "secret-key".into(),
            "https://example.test/api/v1".into(),
            transport,
        );

        let workspace = client.get_workspace("w1").unwrap();
        let request = client.transport.request();

        assert_eq!(workspace.name, "Engineering");
        assert_eq!(request.url, "https://example.test/api/v1/workspaces/w1");
    }

    #[test]
    fn metadata_paths_use_workspace_context() {
        let transport = MockTransport::success(r#"{"id":"p1","name":"Clockify CLI"}"#);
        let client = ClockifyClient::with_base_url(
            "secret-key".into(),
            "https://example.test/api/v1".into(),
            transport,
        );

        let _ = client.get_project("w1", "p1").unwrap();
        let request = client.transport.request();

        assert_eq!(
            request.url,
            "https://example.test/api/v1/workspaces/w1/projects/p1"
        );
    }

    #[test]
    fn create_task_uses_expected_request_shape() {
        let transport =
            MockTransport::success(r#"{"id":"t1","name":"ABC-1: Implement","projectId":"p1"}"#);
        let client = ClockifyClient::with_base_url(
            "secret-key".into(),
            "https://example.test/api/v1".into(),
            transport,
        );

        let task = client.create_task("w1", "p1", "ABC-1: Implement").unwrap();
        let request = client.transport.request();

        assert_eq!(task.id, "t1");
        assert_eq!(request.method, "POST");
        assert_eq!(
            request.url,
            "https://example.test/api/v1/workspaces/w1/projects/p1/tasks"
        );
        assert_eq!(
            request.body.as_deref(),
            Some(r#"{"name":"ABC-1: Implement"}"#)
        );
    }

    #[test]
    fn update_and_delete_entry_use_expected_methods() {
        let transport = MockTransport::success(
            r#"{"id":"e1","workspaceId":"w1","description":"Focus","timeInterval":{"start":"2026-04-23T09:00:00Z"}}"#,
        );
        let client = ClockifyClient::with_base_url(
            "secret-key".into(),
            "https://example.test/api/v1".into(),
            transport,
        );

        let _ = client
            .update_time_entry(
                "w1",
                "e1",
                &serde_json::json!({"start":"2026-04-23T09:00:00Z","end":"2026-04-23T10:00:00Z"}),
            )
            .unwrap();
        let request = client.transport.request();
        assert_eq!(request.method, "PUT");

        let transport = MockTransport::success("{}");
        let client = ClockifyClient::with_base_url(
            "secret-key".into(),
            "https://example.test/api/v1".into(),
            transport,
        );
        client.delete_time_entry("w1", "e1").unwrap();
        let request = client.transport.request();
        assert_eq!(request.method, "DELETE");
    }

    #[test]
    fn current_timer_and_stop_timer_use_expected_paths() {
        let transport = MockTransport::success(
            r#"[{"id":"e1","workspaceId":"w1","userId":"u1","description":"Run","timeInterval":{"start":"2026-04-23T09:00:00Z"}}]"#,
        );
        let client = ClockifyClient::with_base_url(
            "secret-key".into(),
            "https://example.test/api/v1".into(),
            transport,
        );

        let timers = client.get_current_timers("w1").unwrap();
        let request = client.transport.request();
        assert_eq!(timers.len(), 1);
        assert_eq!(
            request.url,
            "https://example.test/api/v1/workspaces/w1/time-entries/status/in-progress"
        );

        let transport = MockTransport::success(
            r#"{"id":"e1","workspaceId":"w1","userId":"u1","description":"Run","timeInterval":{"start":"2026-04-23T09:00:00Z","end":"2026-04-23T10:00:00Z"}}"#,
        );
        let client = ClockifyClient::with_base_url(
            "secret-key".into(),
            "https://example.test/api/v1".into(),
            transport,
        );
        let _ = client
            .stop_timer("w1", "u1", "2026-04-23T10:00:00Z")
            .unwrap();
        let request = client.transport.request();
        assert_eq!(request.method, "PATCH");
        assert_eq!(
            request.url,
            "https://example.test/api/v1/workspaces/w1/user/u1/time-entries"
        );
        assert_eq!(
            request.body.as_deref(),
            Some(r#"{"end":"2026-04-23T10:00:00Z"}"#)
        );
    }

    #[test]
    fn list_time_entries_serializes_filters_to_documented_query_names() {
        let transport = MockTransport::success(
            r#"[{"id":"e1","workspaceId":"w1","userId":"u1","description":"Focus","timeInterval":{"start":"2026-04-23T09:00:00Z"}}]"#,
        );
        let client = ClockifyClient::with_base_url(
            "secret-key".into(),
            "https://example.test/api/v1".into(),
            transport,
        );
        let filters = EntryFilters {
            start: Some("2026-04-23T09:00:00+00:00".into()),
            end: Some("2026-04-23T10:00:00+00:00".into()),
            project: Some("p1".into()),
            task: Some("t1".into()),
            tags: vec!["tag-a".into(), "tag-b".into()],
            description: Some("deep work".into()),
        };

        let entries = client.list_time_entries("w1", "u1", &filters).unwrap();
        let request = client.transport.request();

        assert_eq!(entries.len(), 1);
        assert_eq!(request.method, "GET");
        assert!(request
            .url
            .contains("start=2026-04-23T09%3A00%3A00%2B00%3A00"));
        assert!(request
            .url
            .contains("end=2026-04-23T10%3A00%3A00%2B00%3A00"));
        assert!(request.url.contains("project=p1"));
        assert!(request.url.contains("task=t1"));
        assert!(request.url.contains("description=deep%20work"));
        assert!(request.url.contains("tags=tag-a"));
        assert!(request.url.contains("tags=tag-b"));
    }

    #[test]
    fn transport_errors_are_propagated() {
        let client = ClockifyClient::new(
            "secret-key".into(),
            MockTransport::failure(CfdError::HttpStatus { status: 401 }),
        );

        let error = client.get_current_user().unwrap_err();

        match error {
            CfdError::HttpStatus { status } => assert_eq!(status, 401),
            other => panic!("unexpected error: {other}"),
        }
    }

    #[test]
    fn secrets_are_redacted_in_errors() {
        let client = ClockifyClient::new(
            "secret-key".into(),
            MockTransport::failure(CfdError::transport(
                "upstream rejected X-Api-Key secret-key",
            )),
        );

        let error = client.get_current_user().unwrap_err().to_string();

        assert!(error.contains("[REDACTED]"));
        assert!(!error.contains("secret-key"));
    }

    #[test]
    fn new_uses_base_url_override_when_present() {
        struct EnvGuard;

        impl Drop for EnvGuard {
            fn drop(&mut self) {
                unsafe { std::env::remove_var("CFD_BASE_URL") };
            }
        }

        let _guard = EnvGuard;
        unsafe { std::env::set_var("CFD_BASE_URL", "http://127.0.0.1:12345/api/v1") };

        let transport = MockTransport::success(r#"{"id":"w1","name":"Engineering"}"#);
        let client = ClockifyClient::new("secret-key".into(), transport);

        let _ = client.get_workspace("w1").unwrap();
        let request = client.transport.request();

        assert_eq!(request.url, "http://127.0.0.1:12345/api/v1/workspaces/w1");
    }
}

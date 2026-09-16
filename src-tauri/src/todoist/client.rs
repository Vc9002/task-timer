use serde::{Deserialize, Serialize};
use std::time::Duration;

const API_BASE: &str = "https://api.todoist.com/api/v1";

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct TodoistProject {
    pub id: String,
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub is_deleted: bool,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct TodoistDue {
    pub date: String,
    pub datetime: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct TodoistTask {
    pub id: String,
    #[serde(default)]
    pub project_id: String,
    pub parent_id: Option<String>,
    #[serde(default)]
    pub content: String,
    pub description: Option<String>,
    pub due: Option<TodoistDue>,
    pub priority: Option<i64>,
    #[serde(default)]
    pub checked: bool,
    #[serde(default)]
    pub is_deleted: bool,
    pub completed_at: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct RemoteSync {
    pub projects: Vec<TodoistProject>,
    pub items: Vec<TodoistTask>,
    pub sync_token: String,
    pub full_sync: bool,
}

#[derive(Debug)]
pub enum TodoistApiError {
    InvalidToken,
    RateLimited,
    Network,
    InvalidResponse,
}

impl std::fmt::Display for TodoistApiError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::InvalidToken => {
                "Todoist connection expired or is invalid. Reconnect in Settings."
            }
            Self::RateLimited => "Todoist is rate-limiting requests. Try syncing again shortly.",
            Self::Network => "Todoist couldn't be reached. Your local tasks and timer still work.",
            Self::InvalidResponse => {
                "Todoist returned incomplete data. No local changes were saved. Try syncing again."
            }
        })
    }
}

pub struct TodoistClient {
    http: reqwest::blocking::Client,
    token: String,
    base: String,
}

fn command_succeeded(body: &serde_json::Value, uuid: &str) -> bool {
    body.get("sync_status")
        .and_then(|v| v.get(uuid))
        .and_then(|v| v.as_str())
        == Some("ok")
}

impl TodoistClient {
    pub fn new(token: String) -> Result<Self, TodoistApiError> {
        Ok(Self {
            http: reqwest::blocking::Client::builder()
                .timeout(Duration::from_secs(30))
                .connect_timeout(Duration::from_secs(10))
                .redirect(reqwest::redirect::Policy::none())
                .build()
                .map_err(|_| TodoistApiError::Network)?,
            token,
            base: API_BASE.into(),
        })
    }

    /// Read resources from the incremental Sync API.
    /// Sync returns a full snapshot or cursor-based deltas, not paginated REST lists.
    pub fn fetch(&self, cursor: &str) -> Result<RemoteSync, TodoistApiError> {
        let mut form = reqwest::Url::parse("https://localhost/")
            .map_err(|_| TodoistApiError::InvalidResponse)?;
        form.query_pairs_mut()
            .append_pair("sync_token", cursor)
            .append_pair("resource_types", "[\"projects\",\"items\"]");
        let resp = self
            .http
            .post(format!("{}/sync", self.base))
            .bearer_auth(&self.token)
            .header(
                reqwest::header::CONTENT_TYPE,
                "application/x-www-form-urlencoded",
            )
            .body(form.query().unwrap_or_default().to_owned())
            .send()
            .map_err(|_| TodoistApiError::Network)?;
        match resp.status().as_u16() {
            200 => {}
            401 | 403 => return Err(TodoistApiError::InvalidToken),
            429 => return Err(TodoistApiError::RateLimited),
            _ => return Err(TodoistApiError::Network),
        }
        let result: RemoteSync = resp.json().map_err(|_| TodoistApiError::InvalidResponse)?;
        if result.sync_token.is_empty() || result.sync_token == "*" {
            return Err(TodoistApiError::InvalidResponse);
        }
        Ok(result)
    }

    pub fn test_connection(&self) -> Result<(), TodoistApiError> {
        self.fetch("*").map(|_| ())
    }

    pub fn complete_tasks(&self, commands: &[serde_json::Value]) -> Result<(), TodoistApiError> {
        if commands.is_empty() {
            return Ok(());
        }
        let mut form = reqwest::Url::parse("https://localhost/")
            .map_err(|_| TodoistApiError::InvalidResponse)?;
        form.query_pairs_mut()
            .append_pair("sync_token", "*")
            .append_pair(
                "commands",
                &serde_json::to_string(commands).map_err(|_| TodoistApiError::InvalidResponse)?,
            );
        let response = self
            .http
            .post(format!("{}/sync", self.base))
            .bearer_auth(&self.token)
            .header(
                reqwest::header::CONTENT_TYPE,
                "application/x-www-form-urlencoded",
            )
            .body(form.query().unwrap_or_default().to_owned())
            .send()
            .map_err(|_| TodoistApiError::Network)?;
        match response.status().as_u16() {
            200 => {}
            401 | 403 => return Err(TodoistApiError::InvalidToken),
            429 => return Err(TodoistApiError::RateLimited),
            _ => return Err(TodoistApiError::Network),
        }
        let body: serde_json::Value = response
            .json()
            .map_err(|_| TodoistApiError::InvalidResponse)?;
        for command in commands {
            let uuid = command
                .get("uuid")
                .and_then(|v| v.as_str())
                .ok_or(TodoistApiError::InvalidResponse)?;
            if !command_succeeded(&body, uuid) {
                return Err(TodoistApiError::InvalidResponse);
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{Read, Write};
    fn response(status: u16, body: &'static str) -> Result<RemoteSync, TodoistApiError> {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        let server = std::thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            let mut buf = [0; 4096];
            let count = stream.read(&mut buf).unwrap();
            assert!(String::from_utf8_lossy(&buf[..count]).starts_with("POST /sync"));
            write!(
                stream,
                "HTTP/1.1 {status} Test\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                body.len()
            )
            .unwrap();
        });
        let mut client = TodoistClient::new("test-token".into()).unwrap();
        client.base = format!("http://{address}");
        let result = client.fetch("*");
        server.join().unwrap();
        result
    }
    #[test]
    fn invalid_token() {
        assert!(matches!(
            response(401, "{}"),
            Err(TodoistApiError::InvalidToken)
        ));
    }
    #[test]
    fn rate_limit() {
        assert!(matches!(
            response(429, "{}"),
            Err(TodoistApiError::RateLimited)
        ));
    }
    #[test]
    fn server_failure() {
        assert!(matches!(response(503, "{}"), Err(TodoistApiError::Network)));
    }
    #[test]
    fn incomplete_payload_rejected() {
        assert!(matches!(
            response(200, "{}"),
            Err(TodoistApiError::InvalidResponse)
        ));
    }
    #[test]
    fn full_snapshot() {
        assert!(
            response(
                200,
                r#"{"projects":[],"items":[],"sync_token":"next","full_sync":true}"#
            )
            .unwrap()
            .full_sync
        );
    }
    #[test]
    fn network_failure() {
        let mut client = TodoistClient::new("test-token".into()).unwrap();
        client.base = "http://127.0.0.1:0".into();
        assert!(matches!(client.fetch("*"), Err(TodoistApiError::Network)));
    }

    #[test]
    fn todoist_sync_command_success_is_the_ok_string() {
        let body = serde_json::json!({"sync_status": {"command-1": "ok"}});
        assert!(command_succeeded(&body, "command-1"));
        let legacy = serde_json::json!({"sync_status": {"command-1": {"code": 0}}});
        assert!(!command_succeeded(&legacy, "command-1"));
    }
}

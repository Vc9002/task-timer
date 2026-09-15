use serde::Deserialize;

const API_BASE: &str = "https://api.todoist.com/api/v1";

#[derive(Debug, Clone, Deserialize)]
pub struct TodoistProject {
    pub id: String,
    pub name: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct TodoistDue {
    pub date: String,
    pub datetime: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct TodoistTask {
    pub id: String,
    pub project_id: String,
    pub parent_id: Option<String>,
    pub content: String,
    pub description: Option<String>,
    pub due: Option<TodoistDue>,
    pub priority: Option<i64>,
}

#[derive(Debug, Deserialize)]
struct PagedProjects {
    results: Vec<TodoistProject>,
    next_cursor: Option<String>,
}

#[derive(Debug, Deserialize)]
struct PagedTasks {
    results: Vec<TodoistTask>,
    next_cursor: Option<String>,
}

#[derive(Debug)]
pub enum TodoistApiError {
    InvalidToken,
    RateLimited,
    Network(String),
}

impl std::fmt::Display for TodoistApiError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TodoistApiError::InvalidToken => {
                write!(
                    f,
                    "Todoist connection expired or invalid. Reconnect Todoist in Settings."
                )
            }
            TodoistApiError::RateLimited => {
                write!(
                    f,
                    "Todoist is rate-limiting requests. Try syncing again shortly."
                )
            }
            TodoistApiError::Network(msg) => write!(f, "Could not reach Todoist: {msg}"),
        }
    }
}

pub struct TodoistClient {
    http: reqwest::blocking::Client,
    token: String,
}

impl TodoistClient {
    pub fn new(token: String) -> Self {
        Self {
            http: reqwest::blocking::Client::new(),
            token,
        }
    }

    fn get(&self, path: &str) -> Result<reqwest::blocking::Response, TodoistApiError> {
        let resp = self
            .http
            .get(format!("{API_BASE}{path}"))
            .bearer_auth(&self.token)
            .send()
            .map_err(|e| TodoistApiError::Network(e.to_string()))?;

        match resp.status().as_u16() {
            200..=299 => Ok(resp),
            401 | 403 => Err(TodoistApiError::InvalidToken),
            429 => Err(TodoistApiError::RateLimited),
            other => Err(TodoistApiError::Network(format!("HTTP {other}"))),
        }
    }

    /// Confirms the token works by fetching the first page of projects.
    pub fn test_connection(&self) -> Result<(), TodoistApiError> {
        self.get("/projects").map(|_| ())
    }

    pub fn fetch_all_projects(&self) -> Result<Vec<TodoistProject>, TodoistApiError> {
        let mut all = Vec::new();
        let mut cursor: Option<String> = None;
        loop {
            let path = match &cursor {
                Some(c) => format!("/projects?cursor={c}"),
                None => "/projects".to_string(),
            };
            let page: PagedProjects = self
                .get(&path)?
                .json()
                .map_err(|e| TodoistApiError::Network(e.to_string()))?;
            all.extend(page.results);
            match page.next_cursor {
                Some(next) if !next.is_empty() => cursor = Some(next),
                _ => break,
            }
        }
        Ok(all)
    }

    pub fn fetch_all_tasks(&self) -> Result<Vec<TodoistTask>, TodoistApiError> {
        let mut all = Vec::new();
        let mut cursor: Option<String> = None;
        loop {
            let path = match &cursor {
                Some(c) => format!("/tasks?cursor={c}"),
                None => "/tasks".to_string(),
            };
            let page: PagedTasks = self
                .get(&path)?
                .json()
                .map_err(|e| TodoistApiError::Network(e.to_string()))?;
            all.extend(page.results);
            match page.next_cursor {
                Some(next) if !next.is_empty() => cursor = Some(next),
                _ => break,
            }
        }
        Ok(all)
    }
}

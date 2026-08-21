use serde::{Deserialize, Serialize};

/// Represents a GitHub issue fetched from the API.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitHubIssue {
    pub number: u64,
    pub title: String,
    #[serde(default)]
    pub body: Option<String>,
    pub state: String,
    #[serde(default)]
    pub labels: Vec<GitHubLabel>,
    #[serde(default)]
    pub assignee: Option<GitHubUser>,
    #[serde(default)]
    pub created_at: String,
    #[serde(default)]
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitHubLabel {
    pub name: String,
    #[serde(default)]
    pub color: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitHubUser {
    pub login: String,
}

/// What we return to the frontend after fetching.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IssueImport {
    pub title: String,
    pub description: String,
    pub status: String,
    pub priority: String,
    pub labels: String,
    pub assignee: String,
}

/// Fetch issues from a public GitHub repository.
/// Falls back to anonymous access (rate-limited to 60 req/hr).
pub async fn fetch_issues(owner: &str, repo: &str) -> Result<Vec<IssueImport>, String> {
    let url = format!(
        "https://api.github.com/repos/{}/{}/issues?state=all&per_page=100",
        owner, repo
    );

    let client = reqwest::Client::builder()
        .user_agent("tracera-desktop/0.1.0")
        .build()
        .map_err(|e| e.to_string())?;

    let response = client
        .get(&url)
        .header("Accept", "application/vnd.github+json")
        .send()
        .await
        .map_err(|e| format!("Network error: {}", e))?;

    if !response.status().is_success() {
        return Err(format!("GitHub API error: {}", response.status()));
    }

    let issues: Vec<GitHubIssue> = response
        .json()
        .await
        .map_err(|e| format!("Parse error: {}", e))?;

    let imports: Vec<IssueImport> = issues
        .into_iter()
        // Skip pull requests (they also appear in the issues endpoint)
        .filter(|i| i.body.is_some() || !i.title.starts_with("Merge "))
        .map(|i| {
            let labels = i
                .labels
                .iter()
                .map(|l| l.name.clone())
                .collect::<Vec<_>>()
                .join(", ");

            let assignee = i
                .assignee
                .as_ref()
                .map(|u| u.login.clone())
                .unwrap_or_default();

            // Map GitHub state to Tracera status
            let status = if i.state == "closed" {
                "Done".to_string()
            } else {
                "Backlog".to_string()
            };

            // Try to infer priority from labels
            let priority = if labels.to_lowercase().contains("critical")
                || labels.to_lowercase().contains("p0")
            {
                "Critical".to_string()
            } else if labels.to_lowercase().contains("high")
                || labels.to_lowercase().contains("p1")
            {
                "High".to_string()
            } else if labels.to_lowercase().contains("low")
                || labels.to_lowercase().contains("p3")
            {
                "Low".to_string()
            } else {
                "Medium".to_string()
            };

            let description = i.body.unwrap_or_default();

            IssueImport {
                title: i.title,
                description,
                status,
                priority,
                labels,
                assignee,
            }
        })
        .collect();

    Ok(imports)
}

use serde::{Deserialize, Serialize};
use std::sync::Mutex;
use std::time::{Duration, Instant};

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

const CACHE_TTL_SECS: u64 = 60;

static REPOS: &[&str] = &[
    "KooshaPari/forgecode",
    "KooshaPari/nanovms",
    "KooshaPari/helios-cli",
];

// ---------------------------------------------------------------------------
// Domain structs (what the frontend consumes)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RepoStats {
    pub name: String,
    pub owner: String,
    pub stars: u32,
    pub forks: u32,
    pub open_issues: u32,
    pub last_push: String,
    pub default_branch: String,
    pub description: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CIStatus {
    pub repo: String,
    pub status: String,
    pub conclusion: String,
    pub run_id: u64,
    pub url: String,
    pub branch: String,
    pub commit: String,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PullRequest {
    pub number: u32,
    pub title: String,
    pub author: String,
    pub status: String,
    pub created_at: String,
    pub url: String,
    pub repo: String,
}

// ---------------------------------------------------------------------------
// GitHub API response structs (mirrors JSON shape with Option fields)
// ---------------------------------------------------------------------------

#[derive(Debug, Deserialize)]
struct GhRepoResponse {
    name: Option<String>,
    full_name: Option<String>,
    owner: Option<GhOwner>,
    stargazers_count: Option<u32>,
    forks_count: Option<u32>,
    open_issues_count: Option<u32>,
    pushed_at: Option<String>,
    default_branch: Option<String>,
    description: Option<String>,
}

#[derive(Debug, Deserialize)]
struct GhOwner {
    login: Option<String>,
}

#[derive(Debug, Deserialize)]
struct GhRunsResponse {
    workflow_runs: Option<Vec<GhWorkflowRun>>,
}

#[derive(Debug, Deserialize)]
struct GhWorkflowRun {
    id: Option<u64>,
    name: Option<String>,
    status: Option<String>,
    conclusion: Option<String>,
    html_url: Option<String>,
    head_branch: Option<String>,
    head_sha: Option<String>,
    created_at: Option<String>,
}

#[derive(Debug, Deserialize)]
struct GhPullsResponse {
    number: Option<u32>,
    title: Option<String>,
    user: Option<GhUser>,
    state: Option<String>,
    created_at: Option<String>,
    html_url: Option<String>,
    head: Option<GhPullHead>,
    merged_at: Option<String>,
    closed_at: Option<String>,
}

#[derive(Debug, Deserialize)]
struct GhUser {
    login: Option<String>,
}

#[derive(Debug, Deserialize)]
struct GhPullHead {
    repo: Option<GhPullRepo>,
}

#[derive(Debug, Deserialize)]
struct GhPullRepo {
    full_name: Option<String>,
}

// ---------------------------------------------------------------------------
// Cache
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
struct CacheEntry<T: Clone> {
    data: T,
    timestamp: Instant,
}

struct AppState {
    repo_cache: Mutex<Option<CacheEntry<Vec<RepoStats>>>>,
    ci_cache: Mutex<Option<CacheEntry<Vec<CIStatus>>>>,
    pr_cache: Mutex<Option<CacheEntry<Vec<PullRequest>>>>,
}

fn is_cache_fresh<T: Clone>(entry: &Option<CacheEntry<T>>) -> bool {
    match entry {
        Some(e) => e.timestamp.elapsed() < Duration::from_secs(CACHE_TTL_SECS),
        None => false,
    }
}

// ---------------------------------------------------------------------------
// HTTP helper
// ---------------------------------------------------------------------------

fn build_client() -> reqwest::Client {
    reqwest::Client::builder()
        .user_agent("helios-cli/0.1.0")
        .timeout(Duration::from_secs(15))
        .build()
        .expect("failed to build reqwest client")
}

// ---------------------------------------------------------------------------
// Commands
// ---------------------------------------------------------------------------

#[tauri::command]
async fn get_repo_stats(state: tauri::State<'_, AppState>) -> Result<Vec<RepoStats>, String> {
    // Check cache first
    {
        let cache = state.repo_cache.lock().map_err(|e| e.to_string())?;
        if is_cache_fresh(&*cache) {
            return Ok(cache.as_ref().unwrap().data.clone());
        }
    }

    let client = build_client();
    let mut results = Vec::new();

    for repo_str in REPOS {
        let url = format!("https://api.github.com/repos/{}", repo_str);
        let resp = client
            .get(&url)
            .send()
            .await
            .map_err(|e| format!("HTTP error for {}: {}", repo_str, e))?;

        if !resp.status().is_success() {
            return Err(format!(
                "GitHub API returned {} for {}",
                resp.status(),
                repo_str
            ));
        }

        let gh: GhRepoResponse = resp
            .json()
            .await
            .map_err(|e| format!("JSON parse error for {}: {}", repo_str, e))?;

        results.push(RepoStats {
            name: gh.name.unwrap_or_default(),
            owner: gh.owner.and_then(|o| o.login).unwrap_or_default(),
            stars: gh.stargazers_count.unwrap_or(0),
            forks: gh.forks_count.unwrap_or(0),
            open_issues: gh.open_issues_count.unwrap_or(0),
            last_push: gh.pushed_at.unwrap_or_default(),
            default_branch: gh.default_branch.unwrap_or_default(),
            description: gh.description.unwrap_or_default(),
        });
    }

    // Update cache
    {
        let mut cache = state.repo_cache.lock().map_err(|e| e.to_string())?;
        *cache = Some(CacheEntry {
            data: results.clone(),
            timestamp: Instant::now(),
        });
    }

    Ok(results)
}

#[tauri::command]
async fn get_ci_status(state: tauri::State<'_, AppState>) -> Result<Vec<CIStatus>, String> {
    // Check cache first
    {
        let cache = state.ci_cache.lock().map_err(|e| e.to_string())?;
        if is_cache_fresh(&*cache) {
            return Ok(cache.as_ref().unwrap().data.clone());
        }
    }

    let client = build_client();
    let mut all_runs = Vec::new();

    for repo_str in REPOS {
        let url = format!(
            "https://api.github.com/repos/{}/actions/runs?per_page=5",
            repo_str
        );
        let resp = client
            .get(&url)
            .send()
            .await
            .map_err(|e| format!("HTTP error for {}: {}", repo_str, e))?;

        if !resp.status().is_success() {
            continue;
        }

        let gh: GhRunsResponse = resp
            .json()
            .await
            .map_err(|e| format!("JSON parse error for {}: {}", repo_str, e))?;

        if let Some(runs) = gh.workflow_runs {
            for run in runs {
                all_runs.push(CIStatus {
                    repo: repo_str.to_string(),
                    status: run.status.unwrap_or_default(),
                    conclusion: run.conclusion.unwrap_or_default(),
                    run_id: run.id.unwrap_or(0),
                    url: run.html_url.unwrap_or_default(),
                    branch: run.head_branch.unwrap_or_default(),
                    commit: run.head_sha.unwrap_or_default(),
                    created_at: run.created_at.unwrap_or_default(),
                });
            }
        }
    }

    // Sort by created_at descending (ISO 8601 strings sort lexicographically)
    all_runs.sort_by(|a, b| b.created_at.cmp(&a.created_at));
    all_runs.truncate(20);

    // Update cache
    {
        let mut cache = state.ci_cache.lock().map_err(|e| e.to_string())?;
        *cache = Some(CacheEntry {
            data: all_runs.clone(),
            timestamp: Instant::now(),
        });
    }

    Ok(all_runs)
}

#[tauri::command]
async fn get_recent_prs(state: tauri::State<'_, AppState>) -> Result<Vec<PullRequest>, String> {
    // Check cache first
    {
        let cache = state.pr_cache.lock().map_err(|e| e.to_string())?;
        if is_cache_fresh(&*cache) {
            return Ok(cache.as_ref().unwrap().data.clone());
        }
    }

    let client = build_client();
    let mut all_prs = Vec::new();

    for repo_str in REPOS {
        let url = format!(
            "https://api.github.com/repos/{}/pulls?state=all&per_page=5&sort=updated&direction=desc",
            repo_str
        );
        let resp = client
            .get(&url)
            .send()
            .await
            .map_err(|e| format!("HTTP error for {}: {}", repo_str, e))?;

        if !resp.status().is_success() {
            continue;
        }

        let gh: Vec<GhPullsResponse> = resp
            .json()
            .await
            .map_err(|e| format!("JSON parse error for {}: {}", repo_str, e))?;

        for pr in gh {
            // Determine status from state + merged/closed timestamps
            let status = match pr.state.as_deref() {
                Some("open") => "open".to_string(),
                Some("closed") => {
                    if pr.merged_at.is_some() {
                        "merged".to_string()
                    } else {
                        "closed".to_string()
                    }
                }
                _ => pr.state.unwrap_or_default(),
            };

            all_prs.push(PullRequest {
                number: pr.number.unwrap_or(0),
                title: pr.title.unwrap_or_default(),
                author: pr
                    .user
                    .and_then(|u| u.login)
                    .unwrap_or_default(),
                status,
                created_at: pr.created_at.unwrap_or_default(),
                url: pr.html_url.unwrap_or_default(),
                repo: pr
                    .head
                    .and_then(|h| h.repo)
                    .and_then(|r| r.full_name)
                    .unwrap_or_else(|| repo_str.to_string()),
            });
        }
    }

    // Sort by created_at descending
    all_prs.sort_by(|a, b| b.created_at.cmp(&a.created_at));
    all_prs.truncate(15);

    // Update cache
    {
        let mut cache = state.pr_cache.lock().map_err(|e| e.to_string())?;
        *cache = Some(CacheEntry {
            data: all_prs.clone(),
            timestamp: Instant::now(),
        });
    }

    Ok(all_prs)
}

// ---------------------------------------------------------------------------
// App entry
// ---------------------------------------------------------------------------

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let app_state = AppState {
        repo_cache: Mutex::new(None),
        ci_cache: Mutex::new(None),
        pr_cache: Mutex::new(None),
    };

    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_store::Builder::new().build())
        .plugin(tauri_plugin_notification::init())
        .manage(app_state)
        .invoke_handler(tauri::generate_handler![
            get_repo_stats,
            get_ci_status,
            get_recent_prs,
        ])
        .setup(|app| {
            #[cfg(debug_assertions)]
            {
                let window = app.get_webview_window("main").unwrap();
                window.open_devtools();
            }

            // System tray
            let _tray = tauri::tray::TrayIconBuilder::new("main-tray")
                .tooltip("Tracera - GitHub Dashboard")
                .on_tray_icon_event(|tray_icon, event| {
                    if let tauri::tray::TrayIconEvent::Click {
                        button: tauri::tray::MouseButton::Left,
                        button_state: tauri::tray::MouseButtonState::Up,
                        ..
                    } = event
                    {
                        let app = tray_icon.app_handle();
                        if let Some(window) = app.get_webview_window("main") {
                            let _ = window.show();
                            let _ = window.set_focus();
                        }
                    }
                })
                .build(app)?;

            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

fn main() {
    tracera_lib::run()
}

// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod db;
mod github;
mod labels;
mod sprint;

use db::{CreateIssueRequest, Database, UpdateIssueRequest};
use labels::CreateLabelRequest;
use sprint::{
    AddItemRequest, CreateSprintRequest, UpdateItemStatusRequest,
};
use std::sync::Arc;
use tauri::{
    menu::{MenuBuilder, MenuItemBuilder},
    tray::TrayIconBuilder,
    Manager,
};

/// Tauri-managed application state holding the database handle.
struct AppState {
    db: Arc<Database>,
}

// ── IPC Handlers ──────────────────────────────────────────────────────────────

#[tauri::command]
fn create_issue(
    state: tauri::State<'_, AppState>,
    request: CreateIssueRequest,
) -> Result<db::Issue, String> {
    state
        .db
        .create_issue(request)
        .map_err(|e| e.to_string())
}

#[tauri::command]
fn list_issues(state: tauri::State<'_, AppState>) -> Result<Vec<db::Issue>, String> {
    state.db.list_issues().map_err(|e| e.to_string())
}

#[tauri::command]
fn update_issue(
    state: tauri::State<'_, AppState>,
    request: UpdateIssueRequest,
) -> Result<db::Issue, String> {
    state
        .db
        .update_issue(request)
        .map_err(|e| e.to_string())
}

#[tauri::command]
fn delete_issue(state: tauri::State<'_, AppState>, id: String) -> Result<bool, String> {
    state.db.delete_issue(&id).map_err(|e| e.to_string())
}

#[tauri::command]
async fn import_github_issues(
    state: tauri::State<'_, AppState>,
    owner: String,
    repo: String,
) -> Result<Vec<db::Issue>, String> {
    let imports = github::fetch_issues(&owner, &repo).await?;

    let requests: Vec<CreateIssueRequest> = imports
        .into_iter()
        .map(|imp| CreateIssueRequest {
            title: imp.title,
            description: imp.description,
            status: imp.status,
            priority: imp.priority,
            assignee: imp.assignee,
            labels: imp.labels,
        })
        .collect();

    state
        .db
        .import_issues(requests)
        .map_err(|e| e.to_string())
}

// ── Sprint IPC Handlers ─────────────────────────────────────────────────

#[tauri::command]
fn create_sprint(
    state: tauri::State<'_, AppState>,
    request: CreateSprintRequest,
) -> Result<sprint::Sprint, String> {
    let conn = state.db.get_conn();
    sprint::create_sprint(&conn, request).map_err(|e| e.to_string())
}

#[tauri::command]
fn list_sprints(state: tauri::State<'_, AppState>) -> Result<Vec<sprint::Sprint>, String> {
    let conn = state.db.get_conn();
    sprint::list_sprints(&conn).map_err(|e| e.to_string())
}

#[tauri::command]
fn get_active_sprint(state: tauri::State<'_, AppState>) -> Result<Option<sprint::Sprint>, String> {
    let conn = state.db.get_conn();
    sprint::get_active_sprint(&conn).map_err(|e| e.to_string())
}

#[tauri::command]
fn activate_sprint(
    state: tauri::State<'_, AppState>,
    sprint_id: String,
) -> Result<(), String> {
    let conn = state.db.get_conn();
    sprint::activate_sprint(&conn, &sprint_id).map_err(|e| e.to_string())
}

#[tauri::command]
fn close_sprint(
    state: tauri::State<'_, AppState>,
    sprint_id: String,
) -> Result<(), String> {
    let conn = state.db.get_conn();
    sprint::close_sprint(&conn, &sprint_id).map_err(|e| e.to_string())
}

#[tauri::command]
fn add_to_sprint(
    state: tauri::State<'_, AppState>,
    request: AddItemRequest,
) -> Result<sprint::SprintItem, String> {
    let conn = state.db.get_conn();
    sprint::add_item_to_sprint(&conn, request).map_err(|e| e.to_string())
}

#[tauri::command]
fn remove_sprint_item(
    state: tauri::State<'_, AppState>,
    item_id: String,
) -> Result<bool, String> {
    let conn = state.db.get_conn();
    sprint::remove_item_from_sprint(&conn, &item_id).map_err(|e| e.to_string())
}

#[tauri::command]
fn update_sprint_item(
    state: tauri::State<'_, AppState>,
    request: UpdateItemStatusRequest,
) -> Result<sprint::SprintItem, String> {
    let conn = state.db.get_conn();
    sprint::update_item_status(&conn, request).map_err(|e| e.to_string())
}

#[tauri::command]
fn get_sprint_items(
    state: tauri::State<'_, AppState>,
    sprint_id: String,
) -> Result<Vec<sprint::SprintItem>, String> {
    let conn = state.db.get_conn();
    sprint::get_sprint_items(&conn, &sprint_id).map_err(|e| e.to_string())
}

#[tauri::command]
fn calculate_velocity(
    state: tauri::State<'_, AppState>,
    num_sprints: i32,
) -> Result<Vec<sprint::VelocityData>, String> {
    let conn = state.db.get_conn();
    sprint::calculate_velocity(&conn, num_sprints).map_err(|e| e.to_string())
}

#[tauri::command]
fn get_sprint_burndown(
    state: tauri::State<'_, AppState>,
    sprint_id: String,
) -> Result<sprint::BurndownData, String> {
    let conn = state.db.get_conn();
    sprint::get_sprint_burndown(&conn, &sprint_id).map_err(|e| e.to_string())
}

// ── Label IPC Handlers ──────────────────────────────────────────────────

#[tauri::command]
fn create_label(
    state: tauri::State<'_, AppState>,
    request: CreateLabelRequest,
) -> Result<labels::Label, String> {
    let conn = state.db.get_conn();
    labels::create_label(&conn, request).map_err(|e| e.to_string())
}

#[tauri::command]
fn list_labels(state: tauri::State<'_, AppState>) -> Result<Vec<labels::Label>, String> {
    let conn = state.db.get_conn();
    labels::list_labels(&conn).map_err(|e| e.to_string())
}

#[tauri::command]
fn delete_label(
    state: tauri::State<'_, AppState>,
    id: String,
) -> Result<bool, String> {
    let conn = state.db.get_conn();
    labels::delete_label(&conn, &id).map_err(|e| e.to_string())
}

// ── Main ──────────────────────────────────────────────────────────────────────

fn main() {
    // Resolve a sensible default DB path next to the executable.
    let db_path = dirs_next::data_local_dir()
        .unwrap_or_else(|| std::path::PathBuf::from("."))
        .join("tracera")
        .join("tracera.db");

    // Ensure the parent directory exists.
    if let Some(parent) = db_path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }

    let database = Database::new(db_path.to_str().unwrap_or("tracera.db"))
        .expect("Failed to open database");

    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .manage(AppState {
            db: Arc::new(database),
        })
        .invoke_handler(tauri::generate_handler![
            create_issue,
            list_issues,
            update_issue,
            delete_issue,
            import_github_issues,
            // Sprint
            create_sprint,
            list_sprints,
            get_active_sprint,
            activate_sprint,
            close_sprint,
            add_to_sprint,
            remove_sprint_item,
            update_sprint_item,
            get_sprint_items,
            calculate_velocity,
            get_sprint_burndown,
            // Labels
            create_label,
            list_labels,
            delete_label,
        ])
        .setup(|app| {
            // ── System Tray ────────────────────────────────────────────────
            let show_i = MenuItemBuilder::with_id("show", "Open Tracera").build(app)?;
            let quit_i = MenuItemBuilder::with_id("quit", "Quit").build(app)?;
            let menu = MenuBuilder::new(app).items(&[&show_i, &quit_i]).build()?;

            let _tray = TrayIconBuilder::new()
                .icon(app.default_window_icon().unwrap().clone())
                .menu(&menu)
                .tooltip("Tracera - Project Management")
                .on_menu_event(move |app, event| {
                    match event.id().as_ref() {
                        "show" => {
                            if let Some(win) = app.get_webview_window("main") {
                                let _ = win.show();
                                let _ = win.set_focus();
                            }
                        }
                        "quit" => {
                            std::process::exit(0);
                        }
                        _ => {}
                    }
                })
                .on_tray_icon_event(|tray, event| {
                    if let tauri::tray::TrayIconEvent::DoubleClick { .. } = event {
                        let app = tray.app_handle();
                        if let Some(win) = app.get_webview_window("main") {
                            let _ = win.show();
                            let _ = win.set_focus();
                        }
                    }
                })
                .build(app)?;

            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

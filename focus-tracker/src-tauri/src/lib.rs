// File: src-tauri/src/lib.rs

pub mod database;
pub mod report;
pub mod rules;
pub mod tracker;

use std::time::Duration;
use tauri::{
 menu::{Menu, MenuItem},
 tray::{TrayIconBuilder, TrayIconEvent},
 Manager,
};

// SQLite 데이터베이스 경로를 전역 상태로 관리하기 위한 구조체
pub struct DbPath(pub String);

// V4 전역 프로젝트 태그 상태 구조체
pub struct CurrentProject(pub std::sync::Mutex<Option<String>>);

// --- 프론트엔드 통신용 데이터 구조체 정의 ---
#[derive(serde::Serialize, serde::Deserialize)]
pub struct RuleFrontend {
 id: Option<i32>,
 pattern: String,
 match_type: String,
 category: String,
}

#[derive(serde::Serialize)]
pub struct ReportFrontend {
 process_name: String,
 window_title: String,
 category: String,
 total_duration_sec: i32,
 project_tag: Option<String>,
}

// 시간대별 타임라인 데이터 반환용 구조체
#[derive(serde::Serialize)]
pub struct TimelineItem {
 hour: i32,
 category: String,
 duration: i32,
}

// 실시간 활성 카테고리 응답용 구조체
#[derive(serde::Serialize)]
pub struct ActiveCategoryResponse {
    pub category: String,
    pub process_name: String,
    pub window_title: String,
}

// --- Tauri IPC 커맨드 목록 정의 ---

#[tauri::command]
fn add_rule(state: tauri::State<'_, DbPath>, pattern: String, match_type: String, category: String) -> Result<(), String> {
 let conn = rusqlite::Connection::open(&state.0).map_err(|e| e.to_string())?;
 conn.execute(
 "INSERT INTO classification_rules (pattern, match_type, category) VALUES (?, ?, ?)",
 [pattern, match_type, category],
 ).map_err(|e| e.to_string())?;
 Ok(())
}

#[tauri::command]
fn get_rules(state: tauri::State<'_, DbPath>) -> Result<Vec<RuleFrontend>, String> {
 let conn = rusqlite::Connection::open(&state.0).map_err(|e| e.to_string())?;
 let mut stmt = conn.prepare("SELECT id, pattern, match_type, category FROM classification_rules").map_err(|e| e.to_string())?;
 let mut rows = stmt.query([]).map_err(|e| e.to_string())?;
 let mut rules = Vec::new();
 while let Some(row) = rows.next().map_err(|e| e.to_string())? {
 rules.push(RuleFrontend {
 id: Some(row.get(0).map_err(|e| e.to_string())?),
 pattern: row.get(1).map_err(|e| e.to_string())?,
 match_type: row.get(2).map_err(|e| e.to_string())?,
 category: row.get(3).map_err(|e| e.to_string())?,
 });
 }
 Ok(rules)
}

#[tauri::command]
fn delete_rule(state: tauri::State<'_, DbPath>, id: i32) -> Result<(), String> {
 let conn = rusqlite::Connection::open(&state.0).map_err(|e| e.to_string())?;
 conn.execute("DELETE FROM classification_rules WHERE id = ?", [id]).map_err(|e| e.to_string())?;
 Ok(())
}

#[tauri::command]
fn get_report(state: tauri::State<'_, DbPath>, days: i32) -> Result<Vec<ReportFrontend>, String> {
 let conn = rusqlite::Connection::open(&state.0).map_err(|e| e.to_string())?;
 
 let mut stmt = conn.prepare(
 "SELECT process_name, window_title, category, SUM(duration_sec), project_tag
 FROM time_logs
 WHERE created_at >= datetime('now', '-' || ? || ' days')
 GROUP BY process_name, window_title, category, project_tag
 ORDER BY SUM(duration_sec) DESC"
 ).map_err(|e| e.to_string())?;

 let mut rows = stmt.query([days]).map_err(|e| e.to_string())?;
 let mut result = Vec::new();
 while let Some(row) = rows.next().map_err(|e| e.to_string())? {
 result.push(ReportFrontend {
 process_name: row.get(0).map_err(|e| e.to_string())?,
 window_title: row.get(1).map_err(|e| e.to_string())?,
 category: row.get(2).map_err(|e| e.to_string())?,
 total_duration_sec: row.get(3).map_err(|e| e.to_string())?,
 project_tag: row.get(4).map_err(|e| e.to_string())?,
 });
 }
 Ok(result)
}

#[tauri::command]
fn export_report_csv(app: tauri::AppHandle, state: tauri::State<'_, DbPath>, days: i32) -> Result<(), String> {
 use tauri_plugin_dialog::DialogExt;

 let conn = rusqlite::Connection::open(&state.0).map_err(|e| e.to_string())?;
 let items = report::fetch_report(&conn, days).map_err(|e| e.to_string())?;
 
 let current_time: String = conn.query_row("SELECT datetime('now', 'localtime')", [], |r| r.get(0)).unwrap_or_default();
 let csv_content = report::generate_csv_report(&items, days, &current_time);

 app.dialog()
 .file()
 .add_filter("CSV File", &["csv"])
 .set_file_name("focus_report.csv")
 .save_file(move |file_path| {
 if let Some(path) = file_path {
 match path {
 tauri_plugin_dialog::FilePath::Path(path_buf) => {
 let _ = std::fs::write(&path_buf, &csv_content);
 }
 tauri_plugin_dialog::FilePath::Url(url) => {
 if let Ok(path_buf) = url.to_file_path() {
 let _ = std::fs::write(&path_buf, &csv_content);
 }
 }
 }
 }
 });

 Ok(())
}

// 대시보드 인라인 카테고리 직접 수정 기능
#[tauri::command]
fn update_category_and_rule_cmd(state: tauri::State<'_, DbPath>, pattern: String, match_type: String, category: String) -> Result<(), String> {
 let conn = rusqlite::Connection::open(&state.0).map_err(|e| e.to_string())?;
 database::update_category_and_rule(&conn, &pattern, &match_type, &category).map_err(|e| e.to_string())?;
 Ok(())
}

// 대시보드 항목 삭제 및 Ignore 규칙 삽입 대응
#[tauri::command]
fn delete_and_ignore_item_cmd(state: tauri::State<'_, DbPath>, pattern: String, match_type: String, should_ignore: bool) -> Result<(), String> {
 let conn = rusqlite::Connection::open(&state.0).map_err(|e| e.to_string())?;
 database::delete_and_ignore_item(&conn, &pattern, &match_type, should_ignore).map_err(|e| e.to_string())?;
 Ok(())
}

// 전역 프로젝트 세션 태그 셋업 명령
#[tauri::command]
fn set_current_project(state: tauri::State<'_, CurrentProject>, tag: Option<String>) {
 let mut current = state.0.lock().unwrap();
 *current = tag;
}

// Gist 클라우드 전송용 DB 이진 바이너리 획득 명령
#[tauri::command]
fn get_db_bytes(state: tauri::State<'_, DbPath>) -> Result<Vec<u8>, String> {
 std::fs::read(&state.0).map_err(|e| e.to_string())
}

// Gist 클라우드 다운로드 데이터의 로컬 무손실 복구 명령
#[tauri::command]
fn restore_db_bytes(state: tauri::State<'_, DbPath>, bytes: Vec<u8>) -> Result<(), String> {
 std::fs::write(&state.0, bytes).map_err(|e| e.to_string())?;
 Ok(())
}

// 🌟 V5-비동기개선: Windows WebView2 데드락 방지를 위해 async fn 으로 선언합니다.
#[tauri::command]
async fn toggle_focus_warning(
    app: tauri::AppHandle, 
    show: bool, 
    app_name: Option<String>, 
    domain: Option<String>
) -> Result<(), String> {
    if show {
        if app.get_webview_window("focus_warning").is_none() {
            // 현재 감지된 비생산적 창의 좌표를 획득하여 타겟 모니터 계산
            let mut target_monitor = None;
            if let Ok(active_window) = active_win_pos_rs::get_active_window() {
                let x = active_window.position.x;
                let y = active_window.position.y;
                let w = active_window.position.width;
                let h = active_window.position.height;
                let cx = x + w / 2.0;
                let cy = y + h / 2.0;

                if let Ok(monitors) = app.available_monitors() {
                    for monitor in monitors {
                        let m_pos = monitor.position();
                        let m_size = monitor.size();
                        let mx = m_pos.x as f64;
                        let my = m_pos.y as f64;
                        let mw = m_size.width as f64;
                        let mh = m_size.height as f64;

                        // 마우스/창 중심이 모니터 영역 내에 있는지 검증
                        if cx >= mx && cx <= mx + mw && cy >= my && cy <= my + mh {
                            target_monitor = Some(monitor);
                            break;
                        }
                    }
                }
            }

            // 타겟 모니터를 확보하지 못했다면 주 모니터로 대체 적용 (역참조 * 적용하여 소유권 획득)
            let (pos, size) = if let Some(monitor) = target_monitor {
                (*monitor.position(), *monitor.size())
            } else if let Ok(Some(monitor)) = app.primary_monitor() {
                (*monitor.position(), *monitor.size())
            } else {
                return Err("모니터를 감지할 수 없습니다.".to_string());
            };

            let app_encoded = url_encode(app_name.as_deref().unwrap_or("인터넷 브라우저"));
            let domain_encoded = url_encode(domain.as_deref().unwrap_or("비생산적 사이트"));
            let url = format!("index.html#focus_warning?app={}&domain={}", app_encoded, domain_encoded);

            let window = tauri::WebviewWindowBuilder::new(
                &app,
                "focus_warning",
                tauri::WebviewUrl::App(url.into())
            )
            .title("집중 경고")
            .position(pos.x as f64, pos.y as f64)
            .inner_size(size.width as f64, size.height as f64)
            .resizable(false)
            .always_on_top(true) // 🌟 항상 위에 그리지만 클릭을 관통시키므로 100% 안전합니다!
            .decorations(false)
            .transparent(true) // 투명배경 지원 활성화
            .build();

            // 🌟 [핵심 물리 패치]: 윈도우의 마우스 클릭 및 스크롤을 100% 하위 크롬 브라우저로 관통(Ignore cursor events)시킵니다!
            if let Ok(win) = window {
                let _ = win.set_ignore_cursor_events(true);
            }
        }
    } else {
        if let Some(window) = app.get_webview_window("focus_warning") {
            let _ = window.close();
        }
    }
    Ok(())
}

// 24시간 타임라인 그리드 뷰 시각화용 데이터 획득 명령
#[tauri::command]
fn get_timeline(state: tauri::State<'_, DbPath>) -> Result<Vec<TimelineItem>, String> {
 let conn = rusqlite::Connection::open(&state.0).map_err(|e| e.to_string())?;
 
 let mut stmt = conn.prepare(
 "SELECT strftime('%H', created_at, 'localtime') as hr, category, SUM(duration_sec)
 FROM time_logs
 WHERE created_at >= date('now', 'localtime')
 GROUP BY hr, category"
 ).map_err(|e| e.to_string())?;
 
 let mut rows = stmt.query([]).map_err(|e| e.to_string())?;
 let mut list = Vec::new();
 while let Some(row) = rows.next().map_err(|e| e.to_string())? {
 let hr_str: String = row.get(0).map_err(|e| e.to_string())?;
 list.push(TimelineItem {
 hour: hr_str.parse::<i32>().unwrap_or(0),
 category: row.get(1).map_err(|e| e.to_string())?,
 duration: row.get(2).map_err(|e| e.to_string())?,
 });
 }
 Ok(list)
}

// 바로 직전에 수집된 포커싱 창의 카테고리를 실시간(on-the-fly)으로 판별하여 획득합니다.
// 🌟 V5-비동기개선: 데이터베이스 준비 및 active_window 확인 병목을 막기 위해 async fn 으로 개선합니다.
#[tauri::command]
async fn get_current_active_category(state: tauri::State<'_, DbPath>) -> Result<ActiveCategoryResponse, String> {
    if let Ok(active_window) = active_win_pos_rs::get_active_window() {
        let title = active_window.title;
        let process = active_window.app_name;

        let conn = rusqlite::Connection::open(&state.0).map_err(|e| e.to_string())?;
        let mut rules = Vec::new();
        if let Ok(mut stmt) = conn.prepare("SELECT pattern, match_type, category FROM classification_rules") {
            if let Ok(mut rows) = stmt.query([]) {
                while let Some(row) = rows.next().unwrap_or(None) {
                    let pattern: String = row.get(0).unwrap_or_default();
                    let match_type: String = row.get(1).unwrap_or_default();
                    let category_str: String = row.get(2).unwrap_or_default();
                    
                    let category = match category_str.as_str() {
                        "PRODUCTIVE" => rules::Category::Productive,
                        "UNPRODUCTIVE" => rules::Category::Unproductive,
                        "IGNORE" => rules::Category::Ignore,
                        _ => rules::Category::Neutral,
                    };
                    rules.push(rules::Rule {
                        id: None,
                        pattern,
                        match_type,
                        category,
                    });
                }
            }
        }

        let category = rules::classify(&title, &process, &rules);
        let cat_str = match category {
            rules::Category::Productive => "PRODUCTIVE",
            rules::Category::Unproductive => "UNPRODUCTIVE",
            rules::Category::Ignore => "IGNORE",
            _ => "NEUTRAL",
        };

        Ok(ActiveCategoryResponse {
            category: cat_str.to_string(),
            process_name: process,
            window_title: title,
        })
    } else {
        Ok(ActiveCategoryResponse {
            category: "NEUTRAL".to_string(),
            process_name: "".to_string(),
            window_title: "".to_string(),
        })
    }
}

// --- Tauri 메인 빌더 구동부 ---

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
 tauri::Builder::default()
 .plugin(tauri_plugin_autostart::init(
 tauri_plugin_autostart::MacosLauncher::LaunchAgent,
 Some(vec!["--hidden"]),
 ))
 .plugin(tauri_plugin_dialog::init())
 // 🌟 [1회성 버그 완치 필터]: 창 닫기 방지 정책을 오직 "main" 대시보드 창에만 적용하도록 격리합니다!
 .on_window_event(|window, event| match event {
 tauri::WindowEvent::CloseRequested { api, .. } => {
     if window.label() == "main" {
         let _ = window.hide();
         api.prevent_close();
     }
 }
 _ => {}
 })
 .setup(|app| {
 let app_dir = app
 .path()
 .app_data_dir()
 .unwrap_or_else(|_| std::env::current_dir().unwrap());
 std::fs::create_dir_all(&app_dir).unwrap();
 let db_path_buf = app_dir.join("focus_tracker.db");
 let db_path = db_path_buf.to_string_lossy().to_string();

 let conn = rusqlite::Connection::open(&db_path).unwrap();
 database::init_db_schema(&conn).unwrap();

 let count_rules: i32 = conn
 .query_row("SELECT COUNT(*) FROM classification_rules", [], |r| r.get(0))
 .unwrap();
 if count_rules == 0 {
 conn.execute(
 "INSERT INTO classification_rules (pattern, match_type, category) VALUES ('YouTube', 'CONTAINS', 'UNPRODUCTIVE')",
 [],
 ).unwrap();
 conn.execute(
 "INSERT INTO classification_rules (pattern, match_type, category) VALUES ('Instagram', 'CONTAINS', 'UNPRODUCTIVE')",
 [],
 ).unwrap();
 conn.execute(
 "INSERT INTO classification_rules (pattern, match_type, category) VALUES ('AI Studio', 'CONTAINS', 'PRODUCTIVE')",
 [],
 ).unwrap();
 conn.execute(
 "INSERT INTO classification_rules (pattern, match_type, category) VALUES ('Codex', 'CONTAINS', 'PRODUCTIVE')",
 [],
 ).unwrap();
 }

 app.manage(DbPath(db_path.clone()));
 app.manage(CurrentProject(std::sync::Mutex::new(None)));

 let quit_i = MenuItem::with_id(app, "quit", "앱 완전 종료", true, None::<&str>)?;
 let show_i = MenuItem::with_id(app, "show", "대시보드 열기", true, None::<&str>)?;
 let menu = Menu::with_items(app, &[&show_i, &quit_i])?;

 let tray = TrayIconBuilder::new()
 .menu(&menu)
 .icon(app.default_window_icon().unwrap().clone())
 .show_menu_on_left_click(true)
 .build(app)?;

 tray.on_menu_event(move |app, event| match event.id.as_ref() {
 "quit" => {
 app.exit(0);
 }
 "show" => {
 if let Some(window) = app.get_webview_window("main") {
 let _ = window.show();
 let _ = window.set_focus();
 }
 }
 _ => {}
 });

 tray.on_tray_icon_event(|tray, event| {
 if let TrayIconEvent::Click {
 button: tauri::tray::MouseButton::Left,
 ..
 } = event
 {
 let app = tray.app_handle();
 if let Some(window) = app.get_webview_window("main") {
 let _ = window.show();
 let _ = window.set_focus();
 }
 }
 });

 let db_path_clone = db_path.clone();
 let app_handle = app.handle().clone();
 
 std::thread::spawn(move || loop {
 std::thread::sleep(Duration::from_secs(1));

 if let Ok(active_window) = active_win_pos_rs::get_active_window() {
 let title = active_window.title;
 let process = active_window.app_name;

 let process_lower = process.to_lowercase();
 let is_browser = process_lower.contains("chrome")
 || process_lower.contains("edge")
 || process_lower.contains("msedge")
 || process_lower.contains("brave")
 || process_lower.contains("firefox")
 || process_lower.contains("opera")
 || process_lower.contains("whale");

 if is_browser {
 let current_project = {
 let proj_state = app_handle.state::<CurrentProject>();
 let lock = proj_state.0.lock().unwrap();
 lock.clone()
 };

 if let Ok(conn) = rusqlite::Connection::open(&db_path_clone) {
 let mut rules = Vec::new();
 if let Ok(mut stmt) = conn.prepare("SELECT pattern, match_type, category FROM classification_rules") {
 if let Ok(mut rows) = stmt.query([]) {
 while let Some(row) = rows.next().unwrap_or(None) {
 let pattern: String = row.get(0).unwrap_or_default();
 let match_type: String = row.get(1).unwrap_or_default();
 let category_str: String = row.get(2).unwrap_or_default();
 
 let category = match category_str.as_str() {
 "PRODUCTIVE" => rules::Category::Productive,
 "UNPRODUCTIVE" => rules::Category::Unproductive,
 "IGNORE" => rules::Category::Ignore,
 _ => rules::Category::Neutral,
 };
 rules.push(rules::Rule {
 id: None,
 pattern,
 match_type,
 category,
 });
 }
 }
 }

 let category = rules::classify(&title, &process, &rules);
 if category != rules::Category::Ignore {
 let cat_str = match category {
 rules::Category::Productive => "PRODUCTIVE",
 rules::Category::Unproductive => "UNPRODUCTIVE",
 _ => "NEUTRAL",
 };
 
 let _ = tracker::log_time(
 &conn, 
 &process, 
 &title, 
 cat_str, 
 1, 
 current_project.as_deref()
 );
 }
 }
 }
 }
 });

 Ok(())
 })
 .invoke_handler(tauri::generate_handler![
 add_rule,
 get_rules,
 delete_rule,
 get_report,
 export_report_csv,
 update_category_and_rule_cmd,
 delete_and_ignore_item_cmd,
 set_current_project,
 get_db_bytes,
 restore_db_bytes,
 toggle_focus_warning,
 get_timeline,
 get_current_active_category
 ])
 .run(tauri::generate_context!())
 .expect("error while running tauri application");
}

// ==========================================
// TDD UNIT TESTS & IMPLEMENTATION (Green Phase)
// ==========================================
pub fn url_encode(input: &str) -> String {
    let mut encoded = String::new();
    for b in input.bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                encoded.push(b as char);
            }
            b' ' => {
                encoded.push('+');
            }
            _ => {
                encoded.push_str(&format!("%{:02X}", b));
            }
        }
    }
    encoded
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_url_encode_tdd() {
        assert_eq!(url_encode("유튜브"), "%EC%9C%A0%ED%8A%9C%EB%B8%8C");
    }
}
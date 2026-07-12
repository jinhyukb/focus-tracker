// File: src-tauri/src/tracker.rs

use rusqlite::{Connection, Result};

/// 실시간으로 수집된 사용 시간을 데이터베이스에 누적 기록합니다.
/// [V4 개정]: 동일 날짜에 동일 프로세스, 창, 카테고리 뿐만 아니라 "동일 프로젝트 태그" 조건까지 
/// 정밀하게 일치할 때에만 시간을 합산하고, 다르면 완전히 새로운 행으로 분리하여 저장합니다.
pub fn log_time(
    conn: &Connection, 
    process_name: &str, 
    window_title: &str, 
    category: &str, 
    duration_sec: i32,
    project_tag: Option<&str>, // V4 태그 매개변수 바인딩 적용
) -> Result<()> {
    // 🌟 SQLite의 'IS' 연산자는 바인딩된 값이 NULL(None)이든 문자열(Some)이든 
    // 분기 쿼리문 분할 없이 단일 쿼리로 정확한 등가 비교를 보장합니다.
    let mut stmt = conn.prepare(
        "SELECT id, duration_sec FROM time_logs 
         WHERE process_name = ? 
           AND window_title = ? 
           AND category = ? 
           AND project_tag IS ? 
           AND DATE(created_at) = DATE('now') 
         LIMIT 1"
    )?;
    
    let mut rows = stmt.query(rusqlite::params![
        process_name, 
        window_title, 
        category, 
        project_tag
    ])?;
    
    if let Some(row) = rows.next()? {
        let id: i32 = row.get(0)?;
        let current_duration: i32 = row.get(1)?;
        let new_duration = current_duration + duration_sec;
        
        conn.execute(
            "UPDATE time_logs SET duration_sec = ? WHERE id = ?",
            rusqlite::params![new_duration, id],
        )?;
    } else {
        conn.execute(
            "INSERT INTO time_logs (process_name, window_title, duration_sec, category, project_tag) 
             VALUES (?, ?, ?, ?, ?)",
            rusqlite::params![process_name, window_title, duration_sec, category, project_tag],
        )?;
    }
    
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::database::init_db_schema;

    #[test]
    fn test_log_time_inserts_and_updates() {
        let conn = Connection::open_in_memory().unwrap();
        init_db_schema(&conn).unwrap();

        // 1. 최초 데이터 기록 (태그 없음)
        log_time(&conn, "chrome.exe", "YouTube", "UNPRODUCTIVE", 10, None).unwrap();

        let mut stmt = conn
            .prepare("SELECT duration_sec FROM time_logs WHERE process_name = 'chrome.exe' AND window_title = 'YouTube'")
            .unwrap();
        let mut rows = stmt.query([]).unwrap();
        let row = rows.next().unwrap().expect("로그가 하나 존재해야 합니다.");
        let duration: i32 = row.get(0).unwrap();
        assert_eq!(duration, 10, "최초 기록의 누적 시간은 10초여야 합니다.");

        // 2. 동일 작업 추가 기록
        log_time(&conn, "chrome.exe", "YouTube", "UNPRODUCTIVE", 5, None).unwrap();

        let mut stmt = conn
            .prepare("SELECT duration_sec FROM time_logs WHERE process_name = 'chrome.exe' AND window_title = 'YouTube'")
            .unwrap();
        let mut rows = stmt.query([]).unwrap();
        let row = rows.next().unwrap().expect("동일 행이 계속 존재해야 합니다.");
        let duration: i32 = row.get(0).unwrap();
        assert_eq!(duration, 15, "동일 창의 기록이 합산되어 15초가 되어야 합니다.");
    }

    #[test]
    fn test_log_time_with_project_tags() {
        let conn = Connection::open_in_memory().unwrap();
        init_db_schema(&conn).unwrap();

        // 1. 프로젝트 A 태그를 부착하고 10초 동안 로깅
        log_time(&conn, "chrome.exe", "YouTube", "UNPRODUCTIVE", 10, Some("Project A")).unwrap();

        // 2. 동일한 창이지만 프로젝트 B 태그를 부착하고 5초 동안 로깅
        log_time(&conn, "chrome.exe", "YouTube", "UNPRODUCTIVE", 5, Some("Project B")).unwrap();

        // 3. 태그 부착 없이(None) 20초 동안 로깅
        log_time(&conn, "chrome.exe", "YouTube", "UNPRODUCTIVE", 20, None).unwrap();

        // [어설션 검증] 동일 창이더라도 프로젝트 태그가 다르면 별도의 행으로 총 3개로 격리 저장되어야 합니다.
        let count: i32 = conn
            .query_row("SELECT COUNT(*) FROM time_logs", [], |r| r.get(0))
            .unwrap();
        
        assert_eq!(count, 3, "프로젝트 태그가 다르면 분리되어 총 3개의 별도 행이 존재해야 합니다.");

        // 각각의 분리 수집 시간 무결성 검사
        let duration_a: i32 = conn
            .query_row("SELECT duration_sec FROM time_logs WHERE project_tag = 'Project A'", [], |r| r.get(0))
            .unwrap();
        assert_eq!(duration_a, 10, "프로젝트 A의 시간은 정확히 10초여야 합니다.");

        let duration_b: i32 = conn
            .query_row("SELECT duration_sec FROM time_logs WHERE project_tag = 'Project B'", [], |r| r.get(0))
            .unwrap();
        assert_eq!(duration_b, 5, "프로젝트 B의 시간은 정확히 5초여야 합니다.");

        let duration_none: i32 = conn
            .query_row("SELECT duration_sec FROM time_logs WHERE project_tag IS NULL", [], |r| r.get(0))
            .unwrap();
        assert_eq!(duration_none, 20, "태그가 없는 시간은 정확히 20초여야 합니다.");
    }
}
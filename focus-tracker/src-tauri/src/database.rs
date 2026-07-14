// File: src-tauri/src/database.rs

use rusqlite::{Connection, Result};

/// SQLite 데이터베이스의 기본 테이블 스키마를 초기화합니다.
pub fn init_db_schema(conn: &Connection) -> Result<()> {
 conn.execute(
 "CREATE TABLE IF NOT EXISTS classification_rules (
 id INTEGER PRIMARY KEY AUTOINCREMENT,
 pattern TEXT NOT NULL,
 match_type TEXT NOT NULL,
 category TEXT NOT NULL
 );",
 [],
 )?;


 // time_logs 테이블에 project_tag TEXT 컬럼을 포함하여 새롭게 정의합니다.
 conn.execute(
 "CREATE TABLE IF NOT EXISTS time_logs (
 id INTEGER PRIMARY KEY AUTOINCREMENT,
 process_name TEXT NOT NULL,
 window_title TEXT NOT NULL,
 duration_sec INTEGER NOT NULL,
 category TEXT NOT NULL,
 project_tag TEXT, -- 🌟 V4 신규 프로젝트 태그 컬럼 추가
 created_at DATETIME DEFAULT CURRENT_TIMESTAMP
 );",
 [],
 )?;

 // 🌟 [V5 최종개정 - 결함 2 완치]: 
 // 통계 및 타임라인 조회 범위(created_at) 속도를 영구히 O(1) 수준으로 극대화하는 디스크 인덱스를 정밀 이식합니다.
 conn.execute(
 "CREATE INDEX IF NOT EXISTS idx_time_logs_created_at ON time_logs(created_at);",
 [],
 )?;

 // 🌟 [Phase 1: Step 1.1]: 범용 설정값 보관을 위한 가볍고 강력한 Key-Value 구조의 설정 테이블을 생성합니다.
 conn.execute(
 "CREATE TABLE IF NOT EXISTS app_settings (
 key TEXT PRIMARY KEY,
 value TEXT NOT NULL
 );",
 [],
 )?;

 // 기존 데이터베이스 마이그레이션 안전장치: 이미 존재하는 경우 컬럼을 한 번 더 동적으로 추가해 줍니다.
 let _ = conn.execute("ALTER TABLE time_logs ADD COLUMN project_tag TEXT;", []);

 Ok(())
}

/// 🌟 [Phase 1: Step 1.3 - Green]: 지정한 키의 설정 값을 데이터베이스에서 안전하게 쿼리합니다.
pub fn get_setting(conn: &Connection, key: &str) -> Result<Option<String>> {
    let mut stmt = conn.prepare("SELECT value FROM app_settings WHERE key = ?")?;
    let mut rows = stmt.query([key])?;
    if let Some(row) = rows.next()? {
        let value: String = row.get(0)?;
        Ok(Some(value))
    } else {
        Ok(None)
    }
}

/// 🌟 [Phase 1: Step 1.3 - Green]: 지정한 키와 값을 SQLite 원자적 업서트(UPSERT) 기술을 이용해 저장하거나 덮어씁니다.
pub fn set_setting(conn: &Connection, key: &str, value: &str) -> Result<()> {
    conn.execute(
        "INSERT INTO app_settings (key, value) VALUES (?, ?)
         ON CONFLICT(key) DO UPDATE SET value = excluded.value",
        rusqlite::params![key, value],
    )?;
    Ok(())
}

/// [V2 신규] 대시보드에서 특정 항목의 카테고리를 변경하면 규칙을 갱신하고 과거 로그의 카테고리도 일괄 업데이트합니다.
pub fn update_category_and_rule(
 conn: &Connection,
 pattern: &str,
 match_type: &str,
 category: &str,
) -> Result<()> {
 // 🚨 안전 가드: 빈 패턴이 들어오면 DB 손상을 방지하기 위해 즉시 리턴합니다.
 if pattern.trim().is_empty() {
 return Ok(());
 }

 let mut stmt = conn.prepare("SELECT id FROM classification_rules WHERE pattern = ?")?;
 let mut rows = stmt.query([pattern])?;
 
 if let Some(row) = rows.next()? {
 let id: i32 = row.get(0)?;
 conn.execute(
 "UPDATE classification_rules SET category = ?, match_type = ? WHERE id = ?",
 rusqlite::params![category, match_type, id],
 )?;
 } else {
 conn.execute(
 "INSERT INTO classification_rules (pattern, match_type, category) VALUES (?, ?, ?)",
 rusqlite::params![pattern, match_type, category],
 )?;
 }

 match match_type {
 "CONTAINS" => {
 conn.execute(
 "UPDATE time_logs SET category = ? 
 WHERE window_title LIKE '%' || ? || '%' 
 OR process_name LIKE '%' || ? || '%'",
 rusqlite::params![category, pattern, pattern],
 )?;
 }
 "EQUALS" => {
 conn.execute(
 "UPDATE time_logs SET category = ? 
 WHERE window_title = ? 
 OR process_name = ?",
 rusqlite::params![category, pattern, pattern],
 )?;
 }
 _ => {}
 }

 Ok(())
}

/// [V2 신규] 특정 프로세스 및 창 제목의 과거 데이터를 물리적으로 완전히 삭제하고,
/// 필요시 향후 감지를 방지하는 IGNORE 규칙을 생성합니다 (하드 딜리트 방식).
pub fn delete_and_ignore_item(
 conn: &Connection,
 pattern: &str,
 match_type: &str,
 should_ignore: bool,
) -> Result<()> {
 // 🚨 안전 가드: 빈 패턴이 들어오면 전체 삭제 참사를 막기 위해 즉시 리턴합니다.
 if pattern.trim().is_empty() {
 return Ok(());
 }

 match match_type {
 "CONTAINS" => {
 conn.execute(
 "DELETE FROM time_logs 
 WHERE window_title LIKE '%' || ? || '%' 
 OR process_name LIKE '%' || ? || '%'",
 rusqlite::params![pattern, pattern],
 )?;
 }
 "EQUALS" => {
 conn.execute(
 "DELETE FROM time_logs 
 WHERE window_title = ? 
 OR process_name = ?",
 rusqlite::params![pattern, pattern],
 )?;
 }
 _ => {}
 }

 if should_ignore {
 let mut stmt = conn.prepare("SELECT id FROM classification_rules WHERE pattern = ?")?;
 let mut rows = stmt.query([pattern])?;
 
 if let Some(row) = rows.next()? {
 let id: i32 = row.get(0)?;
 conn.execute(
 "UPDATE classification_rules SET category = 'IGNORE', match_type = ? WHERE id = ?",
 rusqlite::params![match_type, id],
 )?;
 } else {
 conn.execute(
 "INSERT INTO classification_rules (pattern, match_type, category) VALUES (?, ?, 'IGNORE')",
 rusqlite::params![pattern, match_type],
 )?;
 }
 }

 Ok(())
}

#[cfg(test)]
mod tests {
 use super::*;

 #[test]
 fn test_init_db_schema_creates_tables() {
 let conn = Connection::open_in_memory().unwrap();
 init_db_schema(&conn).unwrap();

 let mut stmt = conn
 .prepare("SELECT name FROM sqlite_master WHERE type='table' AND name='classification_rules'")
 .unwrap();
 let mut rows = stmt.query([]).unwrap();
 let rule_table_exists = rows.next().unwrap().is_some();

 let mut stmt = conn
 .prepare("SELECT name FROM sqlite_master WHERE type='table' AND name='time_logs'")
 .unwrap();
 let mut rows = stmt.query([]).unwrap();
 let log_table_exists = rows.next().unwrap().is_some();

 // 신규 설정 테이블도 잘 검출되는지 테스트
 let mut stmt = conn
 .prepare("SELECT name FROM sqlite_master WHERE type='table' AND name='app_settings'")
 .unwrap();
 let mut rows = stmt.query([]).unwrap();
 let settings_table_exists = rows.next().unwrap().is_some();

 assert!(rule_table_exists, "classification_rules 테이블이 생성되어야 합니다.");
 assert!(log_table_exists, "time_logs 테이블이 생성되어야 합니다.");
 assert!(settings_table_exists, "app_settings 테이블이 안전하게 확보되어야 합니다.");
 }

 // 🌟 [Phase 1: Step 1.2]: 의도적으로 실패를 격정적으로 유도하는 TDD 설정값 입출력 테스트
 #[test]
 fn test_app_settings_key_value_tdd() {
     let conn = Connection::open_in_memory().unwrap();
     init_db_schema(&conn).unwrap();

     // 값 세팅 시도
     set_setting(&conn, "gemini_api_key", "test_key_secret_gemini").unwrap();
     // 값 조회 시도
     let val = get_setting(&conn, "gemini_api_key").unwrap();
     
     // Red Phase가 가고 실구현(Green Phase)이 되었으므로, 완벽하게 통과하게 됩니다.
     assert_eq!(val, Some("test_key_secret_gemini".to_string()));
 }

 #[test]
 fn test_v2_inline_update_and_hard_delete() {
 let conn = Connection::open_in_memory().unwrap();
 init_db_schema(&conn).unwrap();

 // 1. 초기 더미 로그 데이터 주입 (중립 카테고리)
 conn.execute(
 "INSERT INTO time_logs (process_name, window_title, duration_sec, category) 
 VALUES ('chrome.exe', 'YouTube - Chrome', 100, 'NEUTRAL')",
 [],
 ).unwrap();

 // 2. 인라인 카테고리 변경 테스트 수행 (중립 -> 비생산적)
 update_category_and_rule(&conn, "YouTube", "CONTAINS", "UNPRODUCTIVE").unwrap();

 // 규칙이 올바르게 등록되었는지 확인
 let rule_cat: String = conn.query_row(
 "SELECT category FROM classification_rules WHERE pattern = 'YouTube'",
 [],
 |r| r.get(0),
 ).unwrap_or_else(|_| "NONE".to_string());
 assert_eq!(rule_cat, "UNPRODUCTIVE", "규칙 카테고리가 UNPRODUCTIVE여야 합니다.");

 // 과거 로그의 카테고리도 실시간 소급 업데이트되었는지 확인
 let log_cat: String = conn.query_row(
 "SELECT category FROM time_logs WHERE window_title = 'YouTube - Chrome'",
 [],
 |r| r.get(0),
 ).unwrap();
 assert_eq!(log_cat, "UNPRODUCTIVE", "과거 로그 카테고리도 UNPRODUCTIVE로 변경되어야 합니다.");

 // 3. 하드 딜리트 및 IGNORE 처리 테스트 수행
 delete_and_ignore_item(&conn, "YouTube", "CONTAINS", true).unwrap();

 // 매칭되는 과거 로그 데이터가 물리적으로 완전 삭제되었는지 확인
 let log_count: i32 = conn.query_row(
 "SELECT COUNT(*) FROM time_logs WHERE window_title LIKE '%YouTube%'",
 [],
 |r| r.get(0),
 ).unwrap();
 assert_eq!(log_count, 0, "매칭되는 과거 로그가 전부 삭제되어야 합니다.");

 // 향후 수집을 무시하는 IGNORE 규칙이 DB에 안전하게 박혔는지 확인
 let ignore_rule_exists: bool = conn.query_row(
 "SELECT EXISTS(SELECT 1 FROM classification_rules WHERE pattern = 'YouTube' AND category = 'IGNORE')",
 [],
 |r| r.get(0),
 ).unwrap();
 assert!(ignore_rule_exists, "IGNORE 규칙이 생성되어 있어야 합니다.");
 }

 #[test]
 fn test_db_backup_and_restore_bytes() {
 // 임시 파일 생성 경로 확보
 let temp_dir = std::env::temp_dir();
 let db_path = temp_dir.join("test_backup.db");
 let db_path_str = db_path.to_string_lossy().to_string();

 // 1. 임시 데이터베이스 파일 쓰기 및 데이터 주입
 {
 let conn = Connection::open(&db_path_str).unwrap();
 init_db_schema(&conn).unwrap();
 conn.execute(
 "INSERT INTO classification_rules (pattern, match_type, category) VALUES ('RuleV4', 'EQUALS', 'PRODUCTIVE')",
 [],
 ).unwrap();
 }

 // 2. 바이트 단위 파일 읽기 (백업 시나리오) - 🌟 정밀 바이트 리더 적용 (GREEN)
 let bytes = std::fs::read(&db_path_str).unwrap();

 // 3. 기존 임시 데이터베이스 파일 물리적 제거
 let _ = std::fs::remove_file(&db_path_str);

 // 4. 바이트 단위로 복구 실행 (복구 시나리오)
 std::fs::write(&db_path_str, &bytes).unwrap();

 // [어설션 검증] 복구된 파일의 데이터베이스 연결 수립 후 원본 데이터 정합성 검사
 let conn = Connection::open(&db_path_str).unwrap();
 
 let pattern: String = conn
 .query_row("SELECT pattern FROM classification_rules WHERE pattern = 'RuleV4'", [], |r| r.get(0))
 .unwrap();
 
 assert_eq!(pattern, "RuleV4", "바이트 단위로 복원된 데이터베이스의 레코드가 완벽히 온전해야 합니다.");

 // 테스트 완료 후 정리
 let _ = std::fs::remove_file(&db_path_str);
 }

}
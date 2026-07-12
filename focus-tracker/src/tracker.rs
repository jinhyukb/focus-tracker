use rusqlite::Connection;
pub fn log_time(conn: &Connection, process: &str, title: &str, duration: u32, category: &str) -> rusqlite::Result<()> {
    // DB에 로그 적재 또는 기존 레코드 시간 누적(UPSERT) 처리...
    Ok(())
}
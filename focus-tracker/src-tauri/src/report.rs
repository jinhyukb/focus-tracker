// File: src-tauri/src/report.rs

use rusqlite::{Connection, Result};

#[derive(Debug, Clone, PartialEq)]
pub struct ReportItem {
    pub process_name: String,
    pub window_title: String,
    pub category: String,
    pub total_duration_sec: i32,
}

/// N일 전부터 현재까지의 시간 기록을 프로세스명, 창 제목, 카테고리별로 그룹화 및 합산하여
/// 총 사용 시간이 높은 순(내림차순)으로 반환합니다.
pub fn fetch_report(conn: &Connection, days: i32) -> Result<Vec<ReportItem>> {
    let mut stmt = conn.prepare(
        "SELECT process_name, window_title, category, SUM(duration_sec) as total_duration
         FROM time_logs
         WHERE created_at >= datetime('now', '-' || ? || ' days')
         GROUP BY process_name, window_title, category
         ORDER BY total_duration DESC"
    )?;

    let mut rows = stmt.query([days])?;
    let mut items = Vec::new();

    while let Some(row) = rows.next()? {
        items.push(ReportItem {
            process_name: row.get(0)?,
            window_title: row.get(1)?,
            category: row.get(2)?,
            total_duration_sec: row.get(3)?,
        });
    }

    Ok(items)
}

/// 초 단위의 시간 값을 비즈니스에 어울리는 가독성 높은 한국어 포맷으로 환산합니다.
fn format_duration(sec: i32) -> String {
    if sec < 60 {
        format!("{}초", sec)
    } else {
        let minutes = sec / 60;
        let seconds = sec % 60;
        if minutes < 60 {
            if seconds == 0 {
                format!("{}분", minutes)
            } else {
                format!("{}분 {}초", minutes, seconds)
            }
        } else {
            let hours = minutes / 60;
            let remain_minutes = minutes % 60;
            if remain_minutes == 0 {
                format!("{}시간", hours)
            } else {
                format!("{}시간 {}분", hours, remain_minutes)
            }
        }
    }
}

/// [V2 개정] 집계된 보고서 아이템 목록을 비즈니스 서식 템플릿 규격의 CSV 문자열로 변환합니다.
pub fn generate_csv_report(items: &[ReportItem], days: i32, current_time: &str) -> String {
    let mut csv = String::new();

    // 1. 보고서 메인 타이틀 행 결정
    let title = if days <= 1 {
        "일일 업무 생산성 보고서"
    } else if days <= 7 {
        "주간 업무 생산성 보고서"
    } else {
        "월간 업무 생산성 보고서"
    };
    csv.push_str(&format!("{}\n", title));

    // 2. 보고서 메타데이터 작성 (생성일 및 대상 범위)
    csv.push_str(&format!(
        "보고서 생성일: {} | 대상 기간: 최근 {}일\n",
        current_time, days
    ));
    csv.push_str("\n");

    // 3. [업무 생산성 요약] 매트릭스 테이블 작성
    csv.push_str("[업무 생산성 요약]\n");
    csv.push_str("총 측정 시간,생산적 시간,비생산적 시간,생산성 점수\n");

    // 요약에 필요한 총합 수집
    let total_tracked_sec: i32 = items.iter().map(|item| item.total_duration_sec).sum();
    let productive_sec: i32 = items
        .iter()
        .filter(|item| item.category == "PRODUCTIVE")
        .map(|item| item.total_duration_sec)
        .sum();
    let unproductive_sec: i32 = items
        .iter()
        .filter(|item| item.category == "UNPRODUCTIVE")
        .map(|item| item.total_duration_sec)
        .sum();

    // 생산성 점수 계산 (생산적 시간 / (생산적+비생산적))
    let productivity_score = if productive_sec + unproductive_sec > 0 {
        ((productive_sec as f64 / (productive_sec + unproductive_sec) as f64) * 100.0).round() as i32
    } else {
        0
    };

    csv.push_str(&format!(
        "{},{},{},{}%\n",
        format_duration(total_tracked_sec),
        format_duration(productive_sec),
        format_duration(unproductive_sec),
        productivity_score
    ));
    csv.push_str("\n");

    // 4. [상세 활동 내역] 상세 데이터 테이블 작성
    csv.push_str("[상세 활동 내역]\n");
    csv.push_str("번호,프로그램/사이트,활성 창 제목,분류,시간(초),포맷팅된 시간,비율(%)\n");

    for (index, item) in items.iter().enumerate() {
        // 해당 행이 전체 기록 시간 중 차지하는 비율 계산
        let ratio = if total_tracked_sec > 0 {
            ((item.total_duration_sec as f64 / total_tracked_sec as f64) * 100.0).round() as i32
        } else {
            0
        };

        csv.push_str(&format!(
            "{},{},{},{},{},{},{}%\n",
            index + 1,
            item.process_name,
            item.window_title,
            item.category,
            item.total_duration_sec,
            format_duration(item.total_duration_sec),
            ratio
        ));
    }

    csv
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::database::init_db_schema;

    #[test]
    fn test_report_aggregation_and_csv_generation() {
        let conn = Connection::open_in_memory().unwrap();
        init_db_schema(&conn).unwrap();

        // 1. 테스트용 과거 로그 가상 적재
        // 오늘(Today) 기록
        conn.execute(
            "INSERT INTO time_logs (process_name, window_title, duration_sec, category, created_at) 
             VALUES ('chrome.exe', 'Google AI Studio', 150, 'PRODUCTIVE', datetime('now'))",
            [],
        ).unwrap();
        conn.execute(
            "INSERT INTO time_logs (process_name, window_title, duration_sec, category, created_at) 
             VALUES ('chrome.exe', 'YouTube', 50, 'UNPRODUCTIVE', datetime('now'))",
            [],
        ).unwrap();

        // 5일 전(5 days ago) 기록
        conn.execute(
            "INSERT INTO time_logs (process_name, window_title, duration_sec, category, created_at) 
             VALUES ('chrome.exe', 'YouTube', 100, 'UNPRODUCTIVE', datetime('now', '-5 days'))",
            [],
        ).unwrap();

        // [검증 A] 최근 1일 단위 리포트 조회
        let report_1d = fetch_report(&conn, 1).unwrap();
        assert_eq!(report_1d.len(), 2, "1일 리포트는 오늘 생성된 2개의 기록만 있어야 합니다.");
        
        // [검증 B] 최근 7일 단위 리포트 조회
        let report_7d = fetch_report(&conn, 7).unwrap();
        assert_eq!(report_7d.len(), 2, "전체 그룹 개수는 2개여야 합니다.");
        
        let youtube_item = report_7d.iter().find(|i| i.window_title == "YouTube").unwrap();
        assert_eq!(youtube_item.total_duration_sec, 150, "7일간의 YouTube 사용 시간은 총 150초로 합산되어야 합니다.");

        // [검증 C] V2 비즈니스 리포트 양식 서식 CSV 내보내기 검증
        let csv_output = generate_csv_report(&report_1d, 1, "2026-07-11 12:45:00");
        let expected_csv = "\
일일 업무 생산성 보고서\n\
보고서 생성일: 2026-07-11 12:45:00 | 대상 기간: 최근 1일\n\
\n\
[업무 생산성 요약]\n\
총 측정 시간,생산적 시간,비생산적 시간,생산성 점수\n\
3분 20초,2분 30초,50초,75%\n\
\n\
[상세 활동 내역]\n\
번호,프로그램/사이트,활성 창 제목,분류,시간(초),포맷팅된 시간,비율(%)\n\
1,chrome.exe,Google AI Studio,PRODUCTIVE,150,2분 30초,75%\n\
2,chrome.exe,YouTube,UNPRODUCTIVE,50,50초,25%\n";
        
        assert_eq!(csv_output, expected_csv, "비즈니스 규격 CSV 출력 서식이 정밀하게 매칭되어야 합니다.");
    }
}
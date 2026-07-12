pub struct AggregateReport { pub title: String, pub total_duration: u32, pub category: String }
pub fn generate_csv(reports: &[AggregateReport]) -> String {
    // CSV Writer를 통해 헤더("프로그램/사이트명,누적시간(초),분류") 및 바디를 CSV 스트링으로 포매팅...
    String::new()
}
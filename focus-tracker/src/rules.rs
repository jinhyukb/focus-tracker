#[derive(Debug, PartialEq, Clone)]
pub enum Category { Productive, Unproductive, Neutral }
pub struct Rule { pub pattern: String, pub match_type: String, pub category: Category }

pub fn classify(title: &str, rules: &[Rule]) -> Category {
    // 타이틀과 규칙 패턴을 대조하는 엔진 로직...
    Category::Neutral
}
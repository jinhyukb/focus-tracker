// File: src-tauri/src/rules.rs

#[derive(Debug, Clone, PartialEq)]
pub enum Category {
    Productive,
    Unproductive,
    Neutral,
    Ignore, // 🌟 V2 무시 제어 기능 지원을 위한 Ignore 열거형 변종 추가
}

#[derive(Debug, Clone)]
pub struct Rule {
    pub id: Option<i32>,
    pub pattern: String,
    pub match_type: String, // "CONTAINS" 또는 "EQUALS"
    pub category: Category,
}

/// 창 제목(window_title)과 실행 파일명(process_name)을 규칙 배열과 대조하여 카테고리를 판별합니다.
/// 일치하는 첫 번째 규칙의 카테고리를 리턴하며, 일치 규칙이 없는 경우 Neutral을 리턴합니다.
pub fn classify(window_title: &str, process_name: &str, rules: &[Rule]) -> Category {
    for rule in rules {
        // 대소문자 구분을 없애기 위해 소문자로 통일하여 비교 처리합니다.
        let title_lower = window_title.to_lowercase();
        let proc_lower = process_name.to_lowercase();
        let pattern_lower = rule.pattern.to_lowercase();

        match rule.match_type.as_str() {
            "CONTAINS" => {
                if title_lower.contains(&pattern_lower) || proc_lower.contains(&pattern_lower) {
                    return rule.category.clone();
                }
            }
            "EQUALS" => {
                if title_lower == pattern_lower || proc_lower == pattern_lower {
                    return rule.category.clone();
                }
            }
            _ => {} // 알 수 없는 매치 타입은 무시
        }
    }
    Category::Neutral
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_classify_matching_rules() {
        // 테스트용 유저 규칙 목록 정의
        let rules = vec![
            Rule {
                id: Some(1),
                pattern: "YouTube".to_string(),
                match_type: "CONTAINS".to_string(),
                category: Category::Unproductive,
            },
            Rule {
                id: Some(2),
                pattern: "AI Studio".to_string(),
                match_type: "CONTAINS".to_string(),
                category: Category::Productive,
            },
            Rule {
                id: Some(3),
                pattern: "notepad.exe".to_string(),
                match_type: "EQUALS".to_string(),
                category: Category::Productive,
            },
            // IGNORE 규칙 매칭 작동 테스트 추가
            Rule {
                id: Some(4),
                pattern: "Focus Tracker".to_string(),
                match_type: "CONTAINS".to_string(),
                category: Category::Ignore,
            },
        ];

        // 1. "YouTube" 가 창 제목에 포함되어 있으므로 Unproductive 판정 요구
        assert_eq!(
            classify("YouTube - Google Chrome", "chrome.exe", &rules),
            Category::Unproductive
        );

        // 2. "AI Studio" 가 창 제목에 포함되어 있으므로 Productive 판정 요구
        assert_eq!(
            classify("Google AI Studio - Brave", "brave.exe", &rules),
            Category::Productive
        );

        // 3. 프로세스명 "notepad.exe"와 EQUALS 매칭이 성립하므로 Productive 판정 요구
        assert_eq!(
            classify("Untitled - Notepad", "notepad.exe", &rules),
            Category::Productive
        );

        // 4. IGNORE 규칙이 올바르게 매칭되는지 확인
        assert_eq!(
            classify("Focus Tracker Dashboard", "focus_tracker.exe", &rules),
            Category::Ignore
        );

        // 5. 어떤 패턴도 매치되지 않는 평범한 타이틀은 Neutral 판정 요구
        assert_eq!(
            classify("My confidential document", "word.exe", &rules),
            Category::Neutral
        );
    }
}
use gestalt_core::ports::outbound::search::SearchResult;

#[path = "../src/tag_filter.rs"]
mod tag_filter;
use tag_filter::{boost_by_tags, extract_tags, filter_by_tags, has_tag};

fn create_mock_result(id: &str, content: &str, path: &str, score: f64) -> SearchResult {
    SearchResult {
        id: id.to_string(),
        path: path.to_string(),
        content: content.to_string(),
        snippet: content.to_string(),
        score,
    }
}

#[test]
fn test_has_tag() {
    let result1 = create_mock_result("1", "This is a [skill] document", "docs/skill.md", 1.0);
    let result2 = create_mock_result("2", "This is a memory document", "docs/[memory].md", 1.0);
    let result3 = create_mock_result("3", "This is a [code] document", "docs/code.md", 1.0);

    assert!(has_tag(&result1, "skill"));
    assert!(!has_tag(&result1, "memory"));
    assert!(!has_tag(&result1, "code"));

    assert!(!has_tag(&result2, "skill"));
    assert!(has_tag(&result2, "memory"));
    assert!(!has_tag(&result2, "code"));

    assert!(!has_tag(&result3, "skill"));
    assert!(!has_tag(&result3, "memory"));
    assert!(has_tag(&result3, "code"));
}

#[test]
fn test_filter_by_tags() {
    let result1 = create_mock_result("1", "This is a [skill] document", "docs/skill.md", 1.0);
    let result2 = create_mock_result(
        "2",
        "This is a [skill] and [memory] document",
        "docs/memory.md",
        1.0,
    );
    let result3 = create_mock_result("3", "This is a [code] document", "docs/code.md", 1.0);

    let results = vec![result1.clone(), result2.clone(), result3.clone()];

    let filtered = filter_by_tags(results.clone(), &["skill"]);
    assert_eq!(filtered.len(), 2);
    assert_eq!(filtered[0].id, "1");
    assert_eq!(filtered[1].id, "2");

    let filtered = filter_by_tags(results.clone(), &["skill", "memory"]);
    assert_eq!(filtered.len(), 1);
    assert_eq!(filtered[0].id, "2");

    let filtered = filter_by_tags(results.clone(), &["code"]);
    assert_eq!(filtered.len(), 1);
    assert_eq!(filtered[0].id, "3");

    let filtered = filter_by_tags(results.clone(), &[]);
    assert_eq!(filtered.len(), 3);
}

#[test]
fn test_boost_by_tags() {
    let result1 = create_mock_result("1", "This is a [skill] document", "docs/skill.md", 1.0);
    let result2 = create_mock_result(
        "2",
        "This is a [skill] and [memory] document",
        "docs/memory.md",
        1.0,
    );
    let result3 = create_mock_result("3", "This is a [code] document", "docs/code.md", 1.0);

    let results = vec![result1.clone(), result2.clone(), result3.clone()];

    let boosted = boost_by_tags(results.clone(), &["skill"], 2.0);
    assert_eq!(boosted.len(), 3);
    assert_eq!(boosted[0].id, "1");
    assert_eq!(boosted[0].score, 2.0);
    assert_eq!(boosted[1].id, "2");
    assert_eq!(boosted[1].score, 2.0);
    assert_eq!(boosted[2].id, "3");
    assert_eq!(boosted[2].score, 1.0);

    let boosted = boost_by_tags(results.clone(), &["skill", "memory"], 2.0);
    assert_eq!(boosted.len(), 3);
    assert_eq!(boosted[0].id, "2");
    assert_eq!(boosted[0].score, 4.0);
    assert_eq!(boosted[1].id, "1");
    assert_eq!(boosted[1].score, 2.0);
    assert_eq!(boosted[2].id, "3");
    assert_eq!(boosted[2].score, 1.0);

    let boosted = boost_by_tags(results.clone(), &[], 2.0);
    assert_eq!(boosted.len(), 3);
    assert_eq!(boosted[0].id, "1");
    assert_eq!(boosted[0].score, 1.0);
    assert_eq!(boosted[1].id, "2");
    assert_eq!(boosted[1].score, 1.0);
    assert_eq!(boosted[2].id, "3");
    assert_eq!(boosted[2].score, 1.0);
}

#[test]
fn test_extract_tags() {
    assert_eq!(
        extract_tags("find all [skill] related to [rust]"),
        vec!["skill", "rust"]
    );
    assert_eq!(
        extract_tags("just a normal search query"),
        Vec::<String>::new()
    );
    assert_eq!(extract_tags("[memory][code] query"), vec!["memory", "code"]);
    assert_eq!(extract_tags("[unfinished tag query"), Vec::<String>::new());
}

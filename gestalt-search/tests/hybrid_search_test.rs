use gestalt_search::{HybridRanker, SearchResultItem};

fn item(id: &str, score: f64) -> SearchResultItem {
    SearchResultItem {
        id: id.to_string(),
        score,
    }
}

#[test]
fn test_rrf_empty_lists() {
    let result = HybridRanker::reciprocal_rank_fusion(vec![], vec![], 60.0);
    assert!(result.is_empty());
}

#[test]
fn test_rrf_single_match() {
    let lexical = vec![item("docA", 0.0)]; // The score in input does not matter, the position/rank matters
    let vector = vec![];

    let result = HybridRanker::reciprocal_rank_fusion(lexical, vector, 60.0);
    assert_eq!(result.len(), 1);
    assert_eq!(result[0].id, "docA");
    assert_eq!(result[0].score, 1.0 / (60.0 + 1.0));
}

#[test]
fn test_rrf_disjoint_documents() {
    let lexical = vec![item("docA", 0.0), item("docB", 0.0)];
    let vector = vec![item("docC", 0.0), item("docD", 0.0)];

    let result = HybridRanker::reciprocal_rank_fusion(lexical, vector, 60.0);
    assert_eq!(result.len(), 4);

    // docA and docC should have rank 1 -> score 1.0 / 61.0
    // docB and docD should have rank 2 -> score 1.0 / 62.0
    // They are sorted descending by score, and then lexicographically by id
    assert_eq!(result[0].id, "docA");
    assert_eq!(result[1].id, "docC");
    assert_eq!(result[2].id, "docB");
    assert_eq!(result[3].id, "docD");

    assert_eq!(result[0].score, 1.0 / 61.0);
    assert_eq!(result[2].score, 1.0 / 62.0);
}

#[test]
fn test_rrf_overlapping_documents() {
    // docB is ranked 1st in lexical and 2nd in vector.
    // docA is ranked 2nd in lexical and 3rd in vector.
    // docC is ranked 1st in vector only.
    let lexical = vec![item("docB", 0.0), item("docA", 0.0)];
    let vector = vec![item("docC", 0.0), item("docB", 0.0), item("docA", 0.0)];

    let result = HybridRanker::reciprocal_rank_fusion(lexical, vector, 60.0);
    assert_eq!(result.len(), 3);

    // docB score: 1.0/61.0 (lexical) + 1.0/62.0 (vector) = 0.016393 + 0.016129 = 0.032522
    // docC score: 1.0/61.0 (vector) = 0.016393
    // docA score: 1.0/62.0 (lexical) + 1.0/63.0 (vector) = 0.016129 + 0.015873 = 0.032002

    assert_eq!(result[0].id, "docB"); // Highest score
    assert_eq!(result[1].id, "docA"); // Second highest score
    assert_eq!(result[2].id, "docC"); // Lowest score

    assert!((result[0].score - (1.0 / 61.0 + 1.0 / 62.0)).abs() < f64::EPSILON);
    assert!((result[1].score - (1.0 / 62.0 + 1.0 / 63.0)).abs() < f64::EPSILON);
    assert!((result[2].score - (1.0 / 61.0)).abs() < f64::EPSILON);
}

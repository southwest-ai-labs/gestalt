use std::collections::HashMap;
use gestalt_router::consensus::quorum::{AgentVote, QuorumEvaluator, QuorumPolicy};

#[test]
fn test_unanimous_quorum() {
    let votes = vec![
        AgentVote { agent_id: "agent_1".to_string(), vote: "approve".to_string() },
        AgentVote { agent_id: "agent_2".to_string(), vote: "approve".to_string() },
        AgentVote { agent_id: "agent_3".to_string(), vote: "approve".to_string() },
    ];
    let result = QuorumEvaluator::evaluate_quorum(&votes, &QuorumPolicy::Unanimous);
    assert_eq!(result.winning_outcome, Some("approve".to_string()));
    assert_eq!(result.agreement_percentage, 100.0);
    assert!(result.dissenting_agent_ids.is_empty());
}

#[test]
fn test_3_agent_split_unanimous_fails() {
    let votes = vec![
        AgentVote { agent_id: "agent_1".to_string(), vote: "approve".to_string() },
        AgentVote { agent_id: "agent_2".to_string(), vote: "approve".to_string() },
        AgentVote { agent_id: "agent_3".to_string(), vote: "reject".to_string() },
    ];
    let result = QuorumEvaluator::evaluate_quorum(&votes, &QuorumPolicy::Unanimous);
    assert_eq!(result.winning_outcome, None);
    assert_eq!(result.agreement_percentage, 0.0);
    assert!(result.dissenting_agent_ids.contains(&"agent_1".to_string()));
    assert!(result.dissenting_agent_ids.contains(&"agent_2".to_string()));
    assert!(result.dissenting_agent_ids.contains(&"agent_3".to_string()));
}

#[test]
fn test_3_agent_split_simple_majority_passes() {
    let votes = vec![
        AgentVote { agent_id: "agent_1".to_string(), vote: "approve".to_string() },
        AgentVote { agent_id: "agent_2".to_string(), vote: "approve".to_string() },
        AgentVote { agent_id: "agent_3".to_string(), vote: "reject".to_string() },
    ];
    let result = QuorumEvaluator::evaluate_quorum(&votes, &QuorumPolicy::SimpleMajority);
    assert_eq!(result.winning_outcome, Some("approve".to_string()));
    // agreement % should be 2/3 ~ 66.666%
    assert!((result.agreement_percentage - 66.666).abs() < 0.1);
    assert_eq!(result.dissenting_agent_ids, vec!["agent_3".to_string()]);
}

#[test]
fn test_3_agent_split_supermajority_passes() {
    let votes = vec![
        AgentVote { agent_id: "agent_1".to_string(), vote: "approve".to_string() },
        AgentVote { agent_id: "agent_2".to_string(), vote: "approve".to_string() },
        AgentVote { agent_id: "agent_3".to_string(), vote: "reject".to_string() },
    ];
    let result = QuorumEvaluator::evaluate_quorum(&votes, &QuorumPolicy::SupermajorityTwoThirds);
    assert_eq!(result.winning_outcome, Some("approve".to_string()));
    assert!((result.agreement_percentage - 66.666).abs() < 0.1);
    assert_eq!(result.dissenting_agent_ids, vec!["agent_3".to_string()]);
}

#[test]
fn test_4_agent_split_supermajority_fails() {
    let votes = vec![
        AgentVote { agent_id: "agent_1".to_string(), vote: "approve".to_string() },
        AgentVote { agent_id: "agent_2".to_string(), vote: "approve".to_string() },
        AgentVote { agent_id: "agent_3".to_string(), vote: "reject".to_string() },
        AgentVote { agent_id: "agent_4".to_string(), vote: "reject".to_string() },
    ];
    let result = QuorumEvaluator::evaluate_quorum(&votes, &QuorumPolicy::SupermajorityTwoThirds);
    assert_eq!(result.winning_outcome, None);
    assert_eq!(result.agreement_percentage, 0.0);
}

#[test]
fn test_weighted_trust_tie_breaker() {
    let mut trust_scores = HashMap::new();
    trust_scores.insert("senior_agent".to_string(), 2.0);
    trust_scores.insert("junior_agent_1".to_string(), 1.0);
    trust_scores.insert("junior_agent_2".to_string(), 0.5);

    let votes = vec![
        AgentVote { agent_id: "senior_agent".to_string(), vote: "approve".to_string() },
        AgentVote { agent_id: "junior_agent_1".to_string(), vote: "reject".to_string() },
        AgentVote { agent_id: "junior_agent_2".to_string(), vote: "reject".to_string() },
    ];

    let result = QuorumEvaluator::evaluate_quorum(&votes, &QuorumPolicy::WeightedTrust(trust_scores));
    
    // total weight = 2.0 + 1.0 + 0.5 = 3.5
    // approve weight = 2.0
    // reject weight = 1.5
    // approve wins the simple majority of weights (2.0 > 1.75)
    
    assert_eq!(result.winning_outcome, Some("approve".to_string()));
    assert_eq!(result.dissenting_agent_ids, vec!["junior_agent_1".to_string(), "junior_agent_2".to_string()]);
    // 2.0 / 3.5 = 57.14%
    assert!((result.agreement_percentage - 57.14).abs() < 0.1);
}

#[test]
fn test_weighted_trust_tie_no_winner() {
    let mut trust_scores = HashMap::new();
    trust_scores.insert("agent_a".to_string(), 1.5);
    trust_scores.insert("agent_b".to_string(), 1.5);

    let votes = vec![
        AgentVote { agent_id: "agent_a".to_string(), vote: "approve".to_string() },
        AgentVote { agent_id: "agent_b".to_string(), vote: "reject".to_string() },
    ];

    let result = QuorumEvaluator::evaluate_quorum(&votes, &QuorumPolicy::WeightedTrust(trust_scores));
    
    // Total weight = 3.0. Both have 1.5. No strict majority.
    assert_eq!(result.winning_outcome, None);
}

#[test]
fn test_empty_votes() {
    let result = QuorumEvaluator::evaluate_quorum(&[], &QuorumPolicy::SimpleMajority);
    assert_eq!(result.total_votes, 0);
    assert_eq!(result.winning_outcome, None);
}

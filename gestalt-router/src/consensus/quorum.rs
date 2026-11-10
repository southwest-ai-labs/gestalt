use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq)]
pub enum QuorumPolicy {
    Unanimous,
    SupermajorityTwoThirds,
    SimpleMajority,
    WeightedTrust(HashMap<String, f64>),
}

#[derive(Debug, Clone)]
pub struct AgentVote {
    pub agent_id: String,
    pub vote: String, // String representation of the vote outcome/option
}

#[derive(Debug, Clone, PartialEq)]
pub struct QuorumDecision {
    pub total_votes: usize,
    pub winning_outcome: Option<String>,
    pub agreement_percentage: f64,
    pub dissenting_agent_ids: Vec<String>,
}

pub struct QuorumEvaluator;

impl QuorumEvaluator {
    pub fn evaluate_quorum(votes: &[AgentVote], policy: &QuorumPolicy) -> QuorumDecision {
        if votes.is_empty() {
            return QuorumDecision {
                total_votes: 0,
                winning_outcome: None,
                agreement_percentage: 0.0,
                dissenting_agent_ids: vec![],
            };
        }

        let total_votes = votes.len();
        
        // Count votes per outcome and total weight per outcome
        let mut outcome_counts: HashMap<String, usize> = HashMap::new();
        let mut outcome_weights: HashMap<String, f64> = HashMap::new();
        let mut total_weight = 0.0;

        for vote in votes {
            *outcome_counts.entry(vote.vote.clone()).or_insert(0) += 1;
            
            let weight = match policy {
                QuorumPolicy::WeightedTrust(trust_scores) => *trust_scores.get(&vote.agent_id).unwrap_or(&1.0),
                _ => 1.0,
            };
            *outcome_weights.entry(vote.vote.clone()).or_insert(0.0) += weight;
            total_weight += weight;
        }

        // Recalculate best outcome properly to handle ties
        let mut best_outcomes = vec![];
        if let QuorumPolicy::WeightedTrust(_) = policy {
            let max_w = outcome_weights.values().cloned().fold(f64::NEG_INFINITY, f64::max);
            for (outcome, weight) in &outcome_weights {
                if (weight - max_w).abs() < f64::EPSILON {
                    best_outcomes.push(outcome.clone());
                }
            }
        } else {
            let max_c = outcome_counts.values().cloned().max().unwrap_or(0);
            for (outcome, count) in &outcome_counts {
                if *count == max_c {
                    best_outcomes.push(outcome.clone());
                }
            }
        }
        
        let best_outcome = if best_outcomes.len() == 1 {
            Some(best_outcomes[0].clone())
        } else {
            None
        };

        if let Some(winning) = best_outcome {
            let (is_quorum_met, agreement_percentage) = match policy {
                QuorumPolicy::Unanimous => {
                    let count = *outcome_counts.get(&winning).unwrap_or(&0);
                    let met = count == total_votes;
                    (met, (count as f64 / total_votes as f64) * 100.0)
                }
                QuorumPolicy::SupermajorityTwoThirds => {
                    let count = *outcome_counts.get(&winning).unwrap_or(&0);
                    // count >= (2/3) * total
                    // 3 * count >= 2 * total
                    let met = 3 * count >= 2 * total_votes;
                    (met, (count as f64 / total_votes as f64) * 100.0)
                }
                QuorumPolicy::SimpleMajority => {
                    let count = *outcome_counts.get(&winning).unwrap_or(&0);
                    // count > total / 2
                    // 2 * count > total
                    let met = 2 * count > total_votes;
                    (met, (count as f64 / total_votes as f64) * 100.0)
                }
                QuorumPolicy::WeightedTrust(_) => {
                    let weight = *outcome_weights.get(&winning).unwrap_or(&0.0);
                    // Simple majority of weights
                    let met = weight > total_weight / 2.0;
                    let percentage = if total_weight > 0.0 {
                        (weight / total_weight) * 100.0
                    } else {
                        0.0
                    };
                    (met, percentage)
                }
            };

            if is_quorum_met {
                let dissenting_agent_ids = votes.iter()
                    .filter(|v| v.vote != winning)
                    .map(|v| v.agent_id.clone())
                    .collect();

                QuorumDecision {
                    total_votes,
                    winning_outcome: Some(winning),
                    agreement_percentage,
                    dissenting_agent_ids,
                }
            } else {
                QuorumDecision {
                    total_votes,
                    winning_outcome: None,
                    agreement_percentage: 0.0,
                    dissenting_agent_ids: votes.iter().map(|v| v.agent_id.clone()).collect(),
                }
            }
        } else {
             QuorumDecision {
                total_votes,
                winning_outcome: None,
                agreement_percentage: 0.0,
                dissenting_agent_ids: votes.iter().map(|v| v.agent_id.clone()).collect(),
            }
        }
    }
}

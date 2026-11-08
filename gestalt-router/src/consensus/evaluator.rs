use std::sync::Arc;
use gestalt_state::StateDb;
use gestalt_ws::BusEvent;
use crate::event_bus::handle_event;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConsensusVerdict {
    Approved,
    Rejected { feedback: String },
    Conflict { reason: String },
}

pub struct ConsensusRunner;

impl ConsensusRunner {
    pub async fn evaluate(
        db: &Arc<StateDb>,
        _primary_diff: &str,
        review_verdict: &ConsensusVerdict,
    ) -> Result<ConsensusVerdict, String> {
        let (event_type, summary) = match review_verdict {
            ConsensusVerdict::Approved => ("consensus_approved".to_string(), "Consensus Approved".to_string()),
            ConsensusVerdict::Rejected { feedback } => {
                ("consensus_rejected".to_string(), format!("Consensus Rejected: {}", feedback))
            }
            ConsensusVerdict::Conflict { reason } => {
                ("consensus_conflict".to_string(), format!("Consensus Conflict: {}", reason))
            }
        };

        let event = BusEvent::new("consensus_runner", event_type, summary);

        handle_event(db, &event, None).await?;

        Ok(review_verdict.clone())
    }
}

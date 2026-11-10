use crate::application::agent::xavier::XavierClient;
use crate::CoreError;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum DecisionStatus {
    Proposed,
    Accepted,
    Superseded,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DecisionRecord {
    pub title: String,
    pub rationale: String,
    pub status: DecisionStatus,
    pub author: String,
    pub insight_id: String,
    pub timestamp: String,
}

pub struct DecisionPromoter {
    // Intentionally empty struct as per common patterns for promoters,
    // though the methods will take a XavierClient.
}

impl DecisionPromoter {
    pub async fn promote_insight_to_decision(
        client: &XavierClient,
        insight_id: &str,
        title: &str,
        rationale: &str,
    ) -> Result<String, CoreError> {
        let timestamp = chrono::Utc::now().to_rfc3339();
        
        let record = DecisionRecord {
            title: title.to_string(),
            rationale: rationale.to_string(),
            status: DecisionStatus::Proposed,
            author: "System".to_string(), // Or take as param if required
            insight_id: insight_id.to_string(),
            timestamp,
        };

        let metadata = serde_json::to_value(&record)
            .map_err(|e| CoreError::Internal(format!("Failed to serialize decision metadata: {}", e)))?;
        
        let decision_text = format!("# Decision: {}\n\n## Rationale\n{}\n\n**Status**: {:?}\n**Insight ID**: {}", record.title, record.rationale, record.status, record.insight_id);

        let id = client.archive_decision(&decision_text, metadata).await
            .map_err(|e| CoreError::Agent(format!("Failed to archive decision: {}", e)))?;

        Ok(id)
    }

    pub async fn list_active_decisions_by_topic(
        client: &XavierClient,
        topic: &str,
    ) -> Result<Vec<DecisionRecord>, CoreError> {
        let query = format!("decision topic: {}", topic);
        let resp = client.search(&query, 50, "hybrid").await
            .map_err(|e| CoreError::Agent(format!("Failed to search decisions by topic: {}", e)))?;
        
        let mut results = Vec::new();
        for r in resp.results {
            if r.path.starts_with("gestalt/decision/") {
                if let Ok(record) = serde_json::from_value::<DecisionRecord>(r.metadata) {
                    if record.status != DecisionStatus::Superseded {
                        results.push(record);
                    }
                }
            }
        }

        Ok(results)
    }

    pub async fn list_active_decisions_by_tag(
        client: &XavierClient,
        tag: &str,
    ) -> Result<Vec<DecisionRecord>, CoreError> {
        let query = format!("decision tag: {}", tag);
        let resp = client.search(&query, 50, "hybrid").await
            .map_err(|e| CoreError::Agent(format!("Failed to search decisions by tag: {}", e)))?;
        
        let mut results = Vec::new();
        for r in resp.results {
            if r.path.starts_with("gestalt/decision/") {
                if let Ok(record) = serde_json::from_value::<DecisionRecord>(r.metadata) {
                    if record.status != DecisionStatus::Superseded {
                        results.push(record);
                    }
                }
            }
        }

        Ok(results)
    }
}

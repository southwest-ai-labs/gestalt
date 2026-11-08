use crate::BusEvent;

/// Represents a filter for a channel subscription.
#[derive(Debug, Clone, PartialEq)]
pub enum SubscriptionFilter {
    /// Matches events where the `event_type` equals the specified string.
    EventType(String),
    /// Matches events where the `agent` equals the specified string.
    Agent(String),
    /// Matches all events.
    None,
}

impl SubscriptionFilter {
    /// Parse a filter string (e.g. "subscribe:agent=jules" or "subscribe:event_type=checkpoint")
    pub fn parse(s: &str) -> Self {
        if let Some(rest) = s.strip_prefix("subscribe:") {
            if let Some(agent) = rest.strip_prefix("agent=") {
                return Self::Agent(agent.to_string());
            } else if let Some(event_type) = rest.strip_prefix("event_type=") {
                return Self::EventType(event_type.to_string());
            }
        }
        Self::None
    }

    /// Check if a `BusEvent` matches this filter.
    pub fn matches(&self, ev: &BusEvent) -> bool {
        match self {
            Self::EventType(ty) => ev.event_type == *ty,
            Self::Agent(agent) => ev.agent == *agent,
            Self::None => true,
        }
    }
}

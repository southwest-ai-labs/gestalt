use gestalt_ws::filter::SubscriptionFilter;
use gestalt_ws::BusEvent;

#[test]
fn test_subscription_filter_parse() {
    assert_eq!(
        SubscriptionFilter::parse("subscribe:agent=jules"),
        SubscriptionFilter::Agent("jules".to_string())
    );
    assert_eq!(
        SubscriptionFilter::parse("subscribe:event_type=checkpoint"),
        SubscriptionFilter::EventType("checkpoint".to_string())
    );
    assert_eq!(
        SubscriptionFilter::parse("invalid"),
        SubscriptionFilter::None
    );
}

#[test]
fn test_subscription_filter_matches() {
    let ev = BusEvent {
        agent: "jules".to_string(),
        event_type: "checkpoint".to_string(),
        run_id: None,
        project: None,
        state: None,
        summary: "test".to_string(),
        metadata: serde_json::Value::Null,
        ts: "".to_string(),
    };

    let f_agent = SubscriptionFilter::Agent("jules".to_string());
    assert!(f_agent.matches(&ev));

    let f_agent_no = SubscriptionFilter::Agent("hermes".to_string());
    assert!(!f_agent_no.matches(&ev));

    let f_event = SubscriptionFilter::EventType("checkpoint".to_string());
    assert!(f_event.matches(&ev));

    let f_event_no = SubscriptionFilter::EventType("run_started".to_string());
    assert!(!f_event_no.matches(&ev));

    let f_none = SubscriptionFilter::None;
    assert!(f_none.matches(&ev));
}

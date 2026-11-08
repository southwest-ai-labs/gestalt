#[path = "../src/commands/bus_subscribe.rs"]
mod bus_subscribe;

use bus_subscribe::{matches_filter, BusEvent, format_event};

#[test]
fn test_matches_filter() {
    let event1 = BusEvent {
        agent: "test_agent".to_string(),
        event_type: "run_started".to_string(),
        summary: "Run started".to_string(),
        project: None,
        run_id: None,
        ts: None,
    };
    
    let event2 = BusEvent {
        agent: "other_agent".to_string(),
        event_type: "run_finished".to_string(),
        summary: "Run finished".to_string(),
        project: None,
        run_id: None,
        ts: None,
    };

    // No filters
    assert!(matches_filter(&event1, None, None));
    assert!(matches_filter(&event2, None, None));

    // Agent filter
    assert!(matches_filter(&event1, Some("test_agent"), None));
    assert!(!matches_filter(&event2, Some("test_agent"), None));

    // Type filter
    assert!(matches_filter(&event1, None, Some("run_started")));
    assert!(!matches_filter(&event1, None, Some("run_finished")));
    assert!(matches_filter(&event2, None, Some("run_finished")));

    // Both filters
    assert!(matches_filter(&event1, Some("test_agent"), Some("run_started")));
    assert!(!matches_filter(&event1, Some("test_agent"), Some("run_finished")));
}

#[test]
fn test_format_event() {
    let event = BusEvent {
        agent: "agent1".to_string(),
        event_type: "event_t".to_string(),
        summary: "Test summary".to_string(),
        project: None,
        run_id: None,
        ts: Some("2024-01-01T00:00:00Z".to_string()),
    };

    let formatted = format_event(&event);
    
    // Test that the correct substrings are present in the formatted string.
    assert!(formatted.contains("2024-01-01T00:00:00Z"));
    assert!(formatted.contains("agent1"));
    assert!(formatted.contains("event_t"));
    assert!(formatted.contains("Test summary"));
}

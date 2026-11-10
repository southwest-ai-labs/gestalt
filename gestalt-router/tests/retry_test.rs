use gestalt_router::retry::{is_retryable_error, RetryPolicy};

#[test]
fn test_is_retryable_error() {
    assert!(is_retryable_error(429, "Too Many Requests"));
    assert!(is_retryable_error(503, "Service Unavailable"));
    assert!(is_retryable_error(504, "Gateway Timeout"));

    assert!(!is_retryable_error(200, "OK"));
    assert!(!is_retryable_error(400, "Bad Request"));
    assert!(!is_retryable_error(401, "Unauthorized"));
    assert!(!is_retryable_error(404, "Not Found"));
    assert!(!is_retryable_error(500, "Internal Server Error"));
}

#[test]
fn test_retry_policy_exponential_growth_without_jitter() {
    let policy = RetryPolicy::new(5, 100, 5000, 0.0);
    
    assert_eq!(policy.calculate_delay(0), 100);
    assert_eq!(policy.calculate_delay(1), 200);
    assert_eq!(policy.calculate_delay(2), 400);
    assert_eq!(policy.calculate_delay(3), 800);
    assert_eq!(policy.calculate_delay(4), 1600);
    assert_eq!(policy.calculate_delay(5), 3200);
}

#[test]
fn test_retry_policy_max_delay_cap() {
    let policy = RetryPolicy::new(10, 100, 1000, 0.0);
    
    assert_eq!(policy.calculate_delay(0), 100);
    assert_eq!(policy.calculate_delay(1), 200);
    assert_eq!(policy.calculate_delay(2), 400);
    assert_eq!(policy.calculate_delay(3), 800);
    assert_eq!(policy.calculate_delay(4), 1000); // Capped
    assert_eq!(policy.calculate_delay(5), 1000); // Capped
}

#[test]
fn test_retry_policy_with_jitter_bounds() {
    let policy = RetryPolicy::new(5, 100, 5000, 0.2); // 20% jitter
    
    // We should test multiple times due to randomness
    for _ in 0..100 {
        let delay_0 = policy.calculate_delay(0);
        assert_eq!(delay_0, 100, "Initial delay should not have jitter applied in this implementation (or if it does, it needs to follow expected bounds)");
        
        // At attempt 1, base delay is 200. Max jitter is 20% of 200 = 40.
        // So bounds are 160 to 240.
        let delay_1 = policy.calculate_delay(1);
        assert!((160..=240).contains(&delay_1), "Delay 1 {} not in bounds [160, 240]", delay_1);

        // At attempt 2, base delay is 400. Max jitter is 20% of 400 = 80.
        // So bounds are 320 to 480.
        let delay_2 = policy.calculate_delay(2);
        assert!((320..=480).contains(&delay_2), "Delay 2 {} not in bounds [320, 480]", delay_2);
    }
}

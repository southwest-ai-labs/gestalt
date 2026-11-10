use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct RetryPolicy {
    pub max_retries: u32,
    pub initial_delay_ms: u64,
    pub max_delay_ms: u64,
    pub jitter_factor: f64,
}

impl RetryPolicy {
    pub fn new(max_retries: u32, initial_delay_ms: u64, max_delay_ms: u64, jitter_factor: f64) -> Self {
        Self {
            max_retries,
            initial_delay_ms,
            max_delay_ms,
            jitter_factor: jitter_factor.clamp(0.0, 1.0),
        }
    }

    pub fn calculate_delay(&self, attempt: u32) -> u64 {
        if attempt == 0 {
            return self.initial_delay_ms;
        }

        // Exponential backoff
        let base_delay = self.initial_delay_ms.saturating_mul(2u64.saturating_pow(attempt));
        let capped_delay = base_delay.min(self.max_delay_ms);

        // Apply jitter
        if self.jitter_factor > 0.0 {
            let bytes = Uuid::new_v4().into_bytes();
            let mut val = 0u64;
            for byte in bytes.iter().take(8) {
                val = (val << 8) | (*byte as u64);
            }
            let random_float = (val as f64) / (u64::MAX as f64);
            
            // Jitter is +/- (capped_delay * jitter_factor)
            let jitter_amount = (capped_delay as f64 * self.jitter_factor * random_float) as u64;
            
            // Randomly add or subtract the jitter amount
            if bytes[8].is_multiple_of(2) {
                capped_delay.saturating_add(jitter_amount).min(self.max_delay_ms)
            } else {
                capped_delay.saturating_sub(jitter_amount)
            }
        } else {
            capped_delay
        }
    }
}

pub fn is_retryable_error(status_code: u16, _error_msg: &str) -> bool {
    // 429 Too Many Requests (Rate Limit)
    // 503 Service Unavailable
    // 504 Gateway Timeout
    matches!(status_code, 429 | 503 | 504)
}

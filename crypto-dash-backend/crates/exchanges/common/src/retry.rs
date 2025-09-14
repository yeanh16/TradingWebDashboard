use std::time::Duration;
use tokio::time::sleep;
use tracing::debug;

/// Retry configuration
#[derive(Debug, Clone)]
pub struct RetryConfig {
    pub max_attempts: u32,
    pub base_delay: Duration,
    pub max_delay: Duration,
    pub multiplier: f64,
}

impl Default for RetryConfig {
    fn default() -> Self {
        Self {
            max_attempts: 5,
            base_delay: Duration::from_millis(100),
            max_delay: Duration::from_secs(30),
            multiplier: 2.0,
        }
    }
}

/// Exponential backoff with jitter
pub async fn exponential_backoff(attempt: u32, config: &RetryConfig) {
    if attempt == 0 {
        return;
    }

    let delay = calculate_delay(attempt, config);
    debug!("Backing off for {:?} (attempt {})", delay, attempt);
    sleep(delay).await;
}

fn calculate_delay(attempt: u32, config: &RetryConfig) -> Duration {
    let exponential_delay = config.base_delay.as_millis() as f64 * config.multiplier.powi(attempt as i32 - 1);
    let delay_ms = exponential_delay.min(config.max_delay.as_millis() as f64) as u64;
    
    // Add jitter (±25%)
    let jitter_range = delay_ms / 4;
    let jitter = if jitter_range > 0 {
        (rand::random::<u64>() % (2 * jitter_range)) as i64 - jitter_range as i64
    } else {
        0
    };
    
    Duration::from_millis((delay_ms as i64 + jitter).max(0) as u64)
}

/// Retry a future with exponential backoff
pub async fn retry_with_backoff<F, Fut, T, E>(
    mut f: F,
    config: RetryConfig,
) -> Result<T, E>
where
    F: FnMut() -> Fut,
    Fut: std::future::Future<Output = Result<T, E>>,
    E: std::fmt::Debug,
{
    let mut attempts = 0;
    
    loop {
        attempts += 1;
        
        match f().await {
            Ok(result) => return Ok(result),
            Err(e) => {
                if attempts >= config.max_attempts {
                    debug!("Max retry attempts ({}) reached", config.max_attempts);
                    return Err(e);
                }
                
                debug!("Attempt {} failed: {:?}", attempts, e);
                exponential_backoff(attempts, &config).await;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_calculate_delay() {
        let config = RetryConfig::default();
        
        // First attempt should be base delay
        let delay1 = calculate_delay(1, &config);
        assert!(delay1.as_millis() >= 75 && delay1.as_millis() <= 125); // ±25% jitter
        
        // Second attempt should be roughly double
        let delay2 = calculate_delay(2, &config);
        assert!(delay2.as_millis() >= 150 && delay2.as_millis() <= 250);
    }

    #[tokio::test]
    async fn test_retry_success() {
        let mut call_count = 0;
        
        let result = retry_with_backoff(
            || {
                call_count += 1;
                async move {
                    if call_count < 3 {
                        Err("failure")
                    } else {
                        Ok("success")
                    }
                }
            },
            RetryConfig {
                max_attempts: 5,
                base_delay: Duration::from_millis(1),
                max_delay: Duration::from_millis(10),
                multiplier: 2.0,
            },
        ).await;
        
        assert_eq!(result, Ok("success"));
        assert_eq!(call_count, 3);
    }

    #[tokio::test]
    async fn test_retry_failure() {
        let mut call_count = 0;
        
        let result: Result<&str, &str> = retry_with_backoff(
            || {
                call_count += 1;
                async move { Err("always fails") }
            },
            RetryConfig {
                max_attempts: 3,
                base_delay: Duration::from_millis(1),
                max_delay: Duration::from_millis(10),
                multiplier: 2.0,
            },
        ).await;
        
        assert_eq!(result, Err("always fails"));
        assert_eq!(call_count, 3);
    }
}

// Simple random function for jitter when std::random is not available
mod rand {
    use std::cell::Cell;
    use std::time::{SystemTime, UNIX_EPOCH};

    thread_local! {
        static SEED: Cell<u64> = Cell::new(1);
    }

    pub fn random<T>() -> T
    where
        T: From<u64>,
    {
        SEED.with(|seed| {
            let current = seed.get();
            let next = current
                .wrapping_mul(1103515245)
                .wrapping_add(12345)
                .wrapping_add(
                    SystemTime::now()
                        .duration_since(UNIX_EPOCH)
                        .unwrap_or_default()
                        .as_nanos() as u64
                );
            seed.set(next);
            T::from(next)
        })
    }
}
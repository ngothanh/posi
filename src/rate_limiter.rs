use std::sync::{Arc, Mutex};
use std::time::Duration;

use crate::scheduler::Scheduler;

trait RateLimiter {
    fn try_acquire(&self, permits: usize) -> bool;
}

struct TokenBucketRateLimiter {
    rate: Rate,
    permits: Arc<Mutex<usize>>,
    schedulers: Scheduler,
}

impl TokenBucketRateLimiter {
    pub fn new(rate: Rate) -> TokenBucketRateLimiter {
        let permit_num = rate.permit_num;
        let duration = rate.duration;
        let scheduler = Scheduler::new(duration);
        TokenBucketRateLimiter {
            rate,
            permits: Arc::new(Mutex::new(permit_num)),
            schedulers: scheduler,
        }
    }

    pub fn start(&self) {
        let permits_clone = Arc::clone(&self.permits); //clone arc, two arcs point to the same memory
        let rate_clone = self.rate.clone();
        self.schedulers.start(move || {
            let mut available_permits = permits_clone.lock().unwrap();
            *available_permits = std::cmp::min(rate_clone.permit_num, *available_permits + rate_clone.permit_num);
        });

        return;
    }
}

impl RateLimiter for TokenBucketRateLimiter {
    fn try_acquire(&self, permits: usize) -> bool {
        let mut available_permits = self.permits.lock().unwrap();
        if *available_permits < permits {
            false
        } else {
            *available_permits -= permits;
            true
        }
    }
}

#[derive(Clone)]
pub struct Rate {
    permit_num: usize,
    duration: Duration,
}

enum RateLimiterType {
    TokenBucket,
    FixedWindow,
    SlidingWindow,
}

#[cfg(test)]
mod tests {
    use std::thread;
    use std::time::Duration;

    use crate::rate_limiter::{Rate, RateLimiter, TokenBucketRateLimiter};

    #[test]
    fn give_token_bucket_rate_limiter_then_it_protects_the_resource_correctly() {
        //given
        let rate = Rate {
            permit_num: 3,
            duration: Duration::from_secs(5),
        };
        let rate_limiter = TokenBucketRateLimiter::new(rate);
        rate_limiter.start();

        //then
        assert_eq!(rate_limiter.try_acquire(2), true);
        assert_eq!(rate_limiter.try_acquire(2), false);

        thread::sleep(Duration::from_secs(5));

        assert_eq!(rate_limiter.try_acquire(3), true);

        return;
    }
}
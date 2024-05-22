use std::time::Duration;

trait RateLimiter {
    fn try_acquire(&mut self, permits: usize) -> bool;
}

struct TokenBucketRateLimiter {
    rate: Rate,
    permits: usize,
}

impl TokenBucketRateLimiter {
    pub fn new(rate: Rate) -> TokenBucketRateLimiter {
        let permit_num = rate.permit_num;
        TokenBucketRateLimiter {
            rate,
            permits: permit_num,
        }
    }
}

impl RateLimiter for TokenBucketRateLimiter {
    fn try_acquire(&mut self, permits: usize) -> bool {
        if self.permits < permits { return false; }

        self.permits = self.permits - permits;
        return true;
    }
}

pub struct Rate {
    permit_num: usize,
    duration: Duration,
}

enum RateLimiterType {
    TokenBucket,
    FixedWindow,
    SlidingWindow,
}
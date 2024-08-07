use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use crate::log_storage::LogStorage;
use crate::scheduler::Scheduler;

trait RateLimiter {
    fn try_acquire(&self, permits: usize) -> bool;

    fn get_type(&self) -> RateLimiterType;
}

struct RateLimiterFactory {
    rate_limiters: HashMap<RateLimiterType, Box<dyn RateLimiter>>,
}

impl RateLimiterFactory {
    fn new(rate_limiters: Vec<Box<dyn RateLimiter>>) -> Box<RateLimiterFactory> {
        let mut res: HashMap<RateLimiterType, Box<dyn RateLimiter>> = HashMap::new();
        for rate_limiter in rate_limiters {
            res.insert(rate_limiter.get_type(), rate_limiter);
        }

        Box::new(
            RateLimiterFactory {
                rate_limiters: res
            }
        )
    }

    fn get(&self, t: RateLimiterType) -> Option<&Box<dyn RateLimiter>> {
        self.rate_limiters.get(&t)
    }
}

struct TokenBucketRateLimiter {
    rate: Rate,
    permits: Arc<Mutex<usize>>,
    schedulers: Scheduler,
}

struct FixedWindowRateLimiter {
    rate: Rate,
    counter: Arc<Mutex<usize>>,
    window_start: Arc<Mutex<Instant>>,
}

struct SlidingWindowLogRateLimiter {
    rate: Rate,
    log_storage: Arc<Mutex<Box<dyn LogStorage + Send>>>,
}

impl SlidingWindowLogRateLimiter {
    pub fn new(rate: Rate, log_storage: Box<dyn LogStorage + Send>) -> SlidingWindowLogRateLimiter {
        SlidingWindowLogRateLimiter {
            rate,
            log_storage: Arc::new(Mutex::new(log_storage)),
        }
    }
}

impl RateLimiter for SlidingWindowLogRateLimiter {
    fn try_acquire(&self, permits: usize) -> bool {
        let storage = self.log_storage.lock().unwrap();
        storage.store(permits, self.rate.duration);
        let count = storage.count();
        return count <= self.rate.permit_num;
    }

    fn get_type(&self) -> RateLimiterType {
        RateLimiterType::SlidingWindowLog
    }
}

impl FixedWindowRateLimiter {
    pub fn new(rate: Rate) -> FixedWindowRateLimiter {
        FixedWindowRateLimiter {
            rate,
            counter: Arc::new(Mutex::new(0)),
            window_start: Arc::new(Mutex::new(Instant::now())),
        }
    }

    pub fn reset_window(&self) {
        let mut counter = self.counter.lock().unwrap();
        let mut window_start = self.window_start.lock().unwrap();
        *counter = 0;
        *window_start = Instant::now()
    }
}

impl RateLimiter for FixedWindowRateLimiter {
    fn try_acquire(&self, permits: usize) -> bool {
        let now = Instant::now();
        let window_start = self.window_start.lock().unwrap();
        let duration = self.rate.duration;
        if now.duration_since(*window_start) > duration {
            drop(window_start);
            self.reset_window();
        }

        let permit_num = self.rate.permit_num;
        let mut cur_counter = self.counter.lock().unwrap();
        if *cur_counter + permits > permit_num {
            return false;
        }

        *cur_counter += permits;
        true
    }

    fn get_type(&self) -> RateLimiterType {
        RateLimiterType::FixedWindow
    }
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

    fn get_type(&self) -> RateLimiterType {
        RateLimiterType::TokenBucket
    }
}

#[derive(Clone)]
pub struct Rate {
    permit_num: usize,
    duration: Duration,
}

#[derive(Eq, Hash, PartialEq)]
enum RateLimiterType {
    TokenBucket,
    FixedWindow,
    SlidingWindowLog,
}

#[cfg(test)]
mod tests {
    use std::thread;
    use std::time::Duration;

    use crate::log_storage::InMemoryLogStorage;
    use crate::rate_limiter::{FixedWindowRateLimiter, Rate, RateLimiter, RateLimiterFactory, RateLimiterType, SlidingWindowLogRateLimiter, TokenBucketRateLimiter};

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

    #[test]
    fn given_fixed_window_rate_limiter_then_it_protects_the_resource_correctly() {
        //given
        let rate = Rate {
            permit_num: 5,
            duration: Duration::from_secs(3),
        };
        let rate_limiter = FixedWindowRateLimiter::new(rate);

        //then
        assert_eq!(rate_limiter.try_acquire(5), true);
        assert_eq!(rate_limiter.try_acquire(1), false);

        thread::sleep(Duration::from_secs(5));
        assert_eq!(rate_limiter.try_acquire(5), true);
    }

    #[test]
    fn given_fixed_window_rate_limiter_then_it_has_problems_at_window_interchanged() {
        //given
        let rate = Rate {
            permit_num: 5,
            duration: Duration::from_secs(3),
        };
        let rate_limiter = FixedWindowRateLimiter::new(rate);
        thread::sleep(Duration::from_secs(2));

        //then
        assert_eq!(rate_limiter.try_acquire(5), true);
        thread::sleep(Duration::from_secs(1));
        assert_eq!(rate_limiter.try_acquire(5), true);
    }

    #[test]
    fn given_sliding_window_log_rate_limiter_then_it_protects_the_resource_correctly() {
        //given
        let rate = Rate {
            permit_num: 5,
            duration: Duration::from_secs(3),
        };
        let storage = InMemoryLogStorage::new(rate.permit_num + 1, rate.duration);
        let rate_limiter = SlidingWindowLogRateLimiter::new(rate, Box::new(storage));

        //then
        assert_eq!(rate_limiter.try_acquire(5), true);
        thread::sleep(Duration::from_secs(3));
        assert_eq!(rate_limiter.try_acquire(5), true);
    }

    #[test]
    fn get_sliding_window_log_rate_limiter_via_factory_then_it_protects_the_resource_correctly() {
        //given
        let rate = Rate {
            permit_num: 5,
            duration: Duration::from_secs(3),
        };
        let log_size = rate.permit_num + 1;
        let duration = rate.duration.clone();
        let storage = InMemoryLogStorage::new(log_size, duration);
        let sliding_window_log_rate_limiter: Box<dyn RateLimiter> = Box::new(SlidingWindowLogRateLimiter::new(rate.clone(), Box::new(storage)));
        let fix_window_rate_limiter: Box<dyn RateLimiter> = Box::new(FixedWindowRateLimiter::new(rate.clone()));
        let token_bucket_rate_limiter: Box<dyn RateLimiter> = Box::new(TokenBucketRateLimiter::new(rate.clone()));
        let rate_limiters = vec![
            sliding_window_log_rate_limiter,
            fix_window_rate_limiter,
            token_bucket_rate_limiter,
        ];

        let factory = RateLimiterFactory::new(rate_limiters);

        //then
        let rate_limiter = factory.get(RateLimiterType::SlidingWindowLog).unwrap();
        assert_eq!(rate_limiter.try_acquire(5), true);
        thread::sleep(Duration::from_secs(3));
        assert_eq!(rate_limiter.try_acquire(5), true);
    }
}
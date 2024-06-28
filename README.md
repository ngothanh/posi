# posi
Rate limit implements in Rust. Posi is like police siren

Sample usage:

```rust
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
        let rate_limiter = factory.get(RateLimiterType::SlidingWindowLog).unwrap();
        rate_limiter.try_acquire(5)  //true
        rate_limiter.try_acquire(1)  //false
        thread::sleep(Duration::from_secs(3));
        rate_limiter.try_acquire(5)  //true
```

      

use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

pub struct Scheduler {
    interval: Duration,
    is_running: Arc<Mutex<bool>>,
}

impl Scheduler {
    pub fn new(interval: Duration) -> Scheduler {
        return Scheduler {
            interval,
            is_running: Arc::new(Mutex::new(false)),
        };
    }

    pub fn start<F>(&self, task: F) -> bool where F: Fn() + Send + 'static {
        let running = Arc::clone(&self.is_running);
        let interval = self.interval;
        if *running.lock().unwrap() {
            return false;
        }

        *self.is_running.lock().unwrap() = true;

        thread::spawn(
            move || {
                while *running.lock().unwrap() {
                    let now = Instant::now();
                    task();
                    let next = now + interval;
                    thread::sleep(next - Instant::now())
                }
            }
        );
        println!("Interval: {:?}", interval); // This line will cause an error

        return true;
    }

    pub fn stop(&self) {
        let mut running = self.is_running.lock().unwrap();
        *running = false;
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::thread;
    use std::time::Duration;
    use crate::scheduler::Scheduler;

    #[test]
    fn given_scheduler_when_start_then_task_was_scheduled_correctly() {
        let interval = Duration::from_millis(100);
        let scheduler = Scheduler::new(interval);
        let counter = Arc::new(AtomicUsize::new(0));
        let clone = Arc::clone(&counter);
        let task = move || {
            clone.fetch_add(1, Ordering::SeqCst);
            return ();
        };
        scheduler.start(task);
        thread::sleep(Duration::from_millis(350));
        scheduler.stop();

        thread::sleep(Duration::from_millis(50));
        let count = counter.load(Ordering::SeqCst);
        assert!(count >= 3, "Task should have run at least 3 times, but ran {} times", count);
    }
}
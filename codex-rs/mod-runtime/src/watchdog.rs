use std::sync::Arc;
use std::sync::atomic::AtomicBool;
use std::sync::atomic::AtomicU64;
use std::sync::atomic::Ordering;
use std::time::Duration;
use std::time::Instant;

pub(crate) struct CpuBudget {
    start: Instant,
    deadline: Arc<AtomicU64>,
    stopped: Arc<AtomicBool>,
    pub exceeded: Arc<AtomicBool>,
}

impl CpuBudget {
    pub(crate) fn new(isolate: v8::IsolateHandle) -> Self {
        let start = Instant::now();
        let deadline = Arc::new(AtomicU64::new(0));
        let stopped = Arc::new(AtomicBool::new(false));
        let exceeded = Arc::new(AtomicBool::new(false));
        let thread_deadline = Arc::clone(&deadline);
        let thread_stopped = Arc::clone(&stopped);
        let thread_exceeded = Arc::clone(&exceeded);
        std::thread::spawn(move || {
            while !thread_stopped.load(Ordering::Acquire) {
                let limit = thread_deadline.load(Ordering::Acquire);
                if limit != 0 && start.elapsed().as_millis() as u64 >= limit {
                    thread_exceeded.store(true, Ordering::Release);
                    isolate.terminate_execution();
                    break;
                }
                std::thread::sleep(Duration::from_millis(5));
            }
        });
        Self {
            start,
            deadline,
            stopped,
            exceeded,
        }
    }

    pub(crate) fn arm(&self, milliseconds: u64) {
        self.deadline.store(
            self.start.elapsed().as_millis() as u64 + milliseconds,
            Ordering::Release,
        );
    }

    pub(crate) fn clear(&self) {
        self.deadline.store(0, Ordering::Release);
    }
}

impl Drop for CpuBudget {
    fn drop(&mut self) {
        self.stopped.store(true, Ordering::Release);
    }
}

//! # Crony: simple cron runner
//! ## Example
//! ```
//! extern crate crony;
//!
//! use crony::{Job, Runner, Schedule};
//! use std::str::FromStr;
//!
//! struct ExampleJob;
//! impl Job for ExampleJob {
//!     fn schedule(&self) -> Schedule {
//!         // Runs every minute
//!         Schedule::from_str("0 * * * * *").unwrap()
//!     }
//!     fn handle(&self) {
//!         println!("Hello, I am cron job running at: {}", self.now());
//!     }
//! }
//!
//! fn main() {
//!     println!("Hello world");
//!     Runner::new().add(Box::new(ExampleJob)).run();
//! }
//!
//! /*
//! Hello world
//! Hello, I am cron job running at: 2020-12-10 16:01:59.740944 UTC
//! Hello, I am cron job running at: 2020-12-10 16:02:59.821043 UTC
//! */
//! ```

extern crate chrono;
extern crate cron;

use chrono::{DateTime, Duration, Utc};
pub use cron::Schedule;
use std::sync::mpsc;
use std::thread::{self, JoinHandle};

pub trait Job: Send + Sync {
    /// Default implementation of is_active method will
    /// make this job always active. If you wish to change
    /// this behaviour write around it in your struct
    fn is_active(&self) -> bool {
        true
    }

    /// Return the schedule instance from your job
    fn schedule(&self) -> Schedule;

    /// This is where your jobs magic happens, define the action that
    /// will happen once the cron start running your job
    fn handle(&self);

    /// Decide wheather or not to start running your job
    fn should_run(&self) -> bool {
        if self.is_active() {
            for item in self.schedule().upcoming(Utc).take(1) {
                let now = Utc::now();
                let difference = item - now;
                if difference <= Duration::milliseconds(100) {
                    return true;
                }
            }
        }

        false
    }

    /// Simple output that will return current time so you don't have to do so
    /// in your job if you wish to display the time of the run.
    fn now(&self) -> DateTime<Utc> {
        Utc::now()
    }
}

/// Runner that will hold all the jobs and will start up the execution
/// and eventually will stop it.
pub struct Runner {
    jobs: Vec<Box<dyn Job>>,
    thread: Option<JoinHandle<()>>,
    running: bool,
    tx: Option<mpsc::Sender<Result<(), ()>>>,
}

impl Runner {
    /// Create new runner
    pub fn new() -> Self {
        Runner {
            jobs: vec![],
            thread: None,
            running: false,
            tx: None,
        }
    }

    /// Add jobs into the runner
    pub fn add(mut self, job: Box<dyn Job>) -> Self {
        if self.running {
            panic!("Cannot push job onto runner once the runner is started!");
        }
        self.jobs.push(job);

        self
    }

    /// Number of jobs ready to start running
    pub fn jobs_to_run(&self) -> usize {
        self.jobs.len()
    }

    /// Start the runner in a spawned thread
    pub fn run(self) -> Self {
        let (tx, rx) = mpsc::channel();
        Self {
            thread: Some(thread::spawn(move || loop {
                match rx.try_recv() {
                    Ok(_) => break,
                    Err(_) => (),
                }

                for job in &self.jobs {
                    if job.should_run() {
                        job.handle();
                    }
                }
                std::thread::sleep(std::time::Duration::from_millis(100));
            })),
            jobs: vec![],
            running: true,
            tx: Some(tx),
        }
    }

    /// Stop the spawned runner
    pub fn stop(mut self) {
        if !self.running {
            return ();
        }
        if let Some(thread) = self.thread.take() {
            if let Some(tx) = self.tx {
                tx.send(Ok(())).unwrap();
            }
            thread
                .join()
                .expect("Could not stop the spawned CronJob thread");
        }
        ()
    }
}

#[cfg(test)]
mod tests {
    use super::{Job, Runner};
    use cron::Schedule;
    use std::str::FromStr;

    struct SomeJob;
    impl Job for SomeJob {
        fn schedule(&self) -> Schedule {
            Schedule::from_str("0 * * * * *").unwrap()
        }

        fn handle(&self) {}
    }
    struct AnotherJob;
    impl Job for AnotherJob {
        fn schedule(&self) -> Schedule {
            Schedule::from_str("0 * * * * *").unwrap()
        }

        fn handle(&self) {}
    }
    #[test]
    fn create_job() {
        let some_job = SomeJob;

        assert_eq!(some_job.handle(), ());
    }

    #[test]
    fn test_adding_jobs_to_runner() {
        let some_job = SomeJob;
        let another_job = AnotherJob;

        let runner = Runner::new()
            .add(Box::new(some_job))
            .add(Box::new(another_job));

        assert_eq!(runner.jobs_to_run(), 2);
    }

    #[test]
    fn test_jobs_are_empty_after_runner_starts() {
        let some_job = SomeJob;
        let another_job = AnotherJob;

        let runner = Runner::new()
            .add(Box::new(some_job))
            .add(Box::new(another_job))
            .run();

        assert_eq!(runner.jobs_to_run(), 0);
    }

    #[test]
    fn test_stopping_the_runner() {
        let some_job = SomeJob;
        let another_job = AnotherJob;

        let runner = Runner::new()
            .add(Box::new(some_job))
            .add(Box::new(another_job))
            .run();

        assert_eq!(runner.stop(), ());
    }
}

//! # Crony: a simple cron runner
//!
//! Use the `Job` trait to create your cron job struct, pass it to the `Runner` and then start it via `run()` method.
//! Runner will spawn new thread where it will start looping through the jobs and will run their handle
//! method once the scheduled time is reached.
//!
//! If your OS has enough threads to spare each job will get its own thread to execute, if not it will be
//! executed in the same thread as the loop but will hold the loop until the job is finished.
//!
//! Please look at the [**`Job trait`**](./trait.Job.html) documentation for more information.
//!
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
#[macro_use]
extern crate log;

use chrono::{DateTime, Duration, Utc};
use lazy_static::lazy_static;
use std::sync::mpsc::{Receiver, Sender};
use std::sync::{mpsc, Arc, Mutex};
use std::thread::{Builder, JoinHandle};

/// Re-export of (cron)[https://crates.io/crates/cron] crate
/// struct for setting the cron schedule time.
/// Read its documentation for more information and options.
pub use cron::Schedule;

lazy_static! {
    /// Singleton instance of a tracker that won't allow
    /// same job to run again while its already running
    /// unless you specificly allow the job to run in
    /// parallel with itself
    pub static ref TRACKER: Mutex<Tracker> = Mutex::new(Tracker::new());
}

pub trait Job: Send + Sync {
    /// Default implementation of is_active method will
    /// make this job always active
    fn is_active(&self) -> bool {
        true
    }

    /// In case your job takes longer to finish and it's scheduled
    /// to start again (while its still running), default behaviour
    /// will skip the next run while one instance is already running.
    /// (if your OS has enough threads, and is spawning a thread for next job)
    ///
    /// To override this behaviour and enable it to run in parallel
    /// with other instances of self, return `true` on this instance.
    fn allow_parallel_runs(&self) -> bool {
        false
    }

    /// Define the run schedule for your job
    fn schedule(&self) -> Schedule;

    /// This is where your jobs magic happens, define the action that
    /// will happen once the cron start running your job
    ///
    /// If this method panics, your entire job will panic and that may
    /// or may not make the whole runner panic. Handle your errors
    /// properly and don't let it panic.
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

/// Struct for marking jobs running
pub struct Tracker(Vec<usize>);

impl Default for Tracker {
    fn default() -> Self {
        Self::new()
    }
}

impl Tracker {
    /// Return new instance of running
    pub fn new() -> Self {
        Tracker(vec![])
    }

    /// Check if id of the job is marked as running
    pub fn running(&self, id: &usize) -> bool {
        self.0.contains(id)
    }

    /// Set job id as running
    pub fn start(&mut self, id: &usize) {
        if !self.running(id) {
            self.0.push(*id);
        }
    }

    /// Unmark the job from running
    pub fn stop(&mut self, id: &usize) {
        if self.running(id) {
            match self.0.iter().position(|&r| r == *id) {
                Some(i) => self.0.remove(i),
                None => 0,
            };
        }
    }
}

/// Runner that will hold all the jobs and will start up the execution
/// and eventually will stop it.
pub struct Runner {
    jobs: Vec<Arc<Box<dyn Job>>>,
    thread: Option<JoinHandle<()>>,
    running: bool,
    tx: Option<mpsc::Sender<Result<(), ()>>>,
}

impl Default for Runner {
    fn default() -> Self {
        Self::new()
    }
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
    ///
    /// **panics** if you try to push a job onto already started runner
    #[allow(clippy::should_implement_trait)]
    pub fn add(mut self, job: Box<dyn Job>) -> Self {
        if self.running {
            panic!("Cannot push job onto runner once the runner is started!");
        }
        self.jobs.push(Arc::new(job));

        self
    }

    /// Number of jobs ready to start running
    pub fn jobs_to_run(&self) -> usize {
        self.jobs.len()
    }

    /// Start the loop and job execution
    pub fn run(self) -> Self {
        if self.jobs.is_empty() {
            return self;
        }

        let (thread, tx) = spawn(self);

        Self {
            thread,
            jobs: vec![],
            running: true,
            tx,
        }
    }

    /// Stop the spawned runner
    pub fn stop(mut self) {
        if !self.running {
            return;
        }
        if let Some(thread) = self.thread.take() {
            if let Some(tx) = self.tx {
                match tx.send(Ok(())) {
                    Ok(_) => (),
                    Err(e) => error!("Could not send stop signal to cron runner thread: {}", e),
                };
            }
            thread
                .join()
                .expect("Could not stop the spawned cron runner thread");
        }
    }
}

type TxRx = (Sender<Result<(), ()>>, Receiver<Result<(), ()>>);
type JhTx = (Option<JoinHandle<()>>, Option<Sender<Result<(), ()>>>);

/// Spanw the thread for the runner and return its sender to stop it
fn spawn(runner: Runner) -> JhTx {
    let (tx, rx): TxRx = mpsc::channel();

    match Builder::new()
        .name(String::from("cron-runner-thread"))
        .spawn(move || {
            let jobs = runner.jobs;

            loop {
                if rx.try_recv().is_ok() {
                    info!("Stopping the cron runner thread");
                    break;
                }

                for (id, job) in jobs.iter().enumerate() {
                    let no = (id + 1).to_string();
                    let _job = job.clone();

                    if _job.should_run()
                        && (_job.allow_parallel_runs() || !TRACKER.lock().unwrap().running(&id))
                    {
                        TRACKER.lock().unwrap().start(&id);

                        // Spawning a new thread to run the job
                        match Builder::new()
                            .name(format!("cron-job-thread-{}", &no))
                            .spawn(move || {
                                let now = Utc::now();
                                debug!(
                                    "START: {} --- {}",
                                    format!("cron-job-thread-{}", &no),
                                    now.format("%H:%M:%S%.f")
                                );

                                _job.handle();
                                TRACKER.lock().unwrap().stop(&id);

                                debug!(
                                    "FINISH: {} --- {}",
                                    format!("cron-job-thread-{}", &no),
                                    now.format("%H:%M:%S%.f")
                                );
                            }) {
                            Ok(_) => (),
                            // In case spawning a new thread fails, we'll run it here
                            Err(_) => {
                                let no = (id + 1).to_string();
                                let now = Utc::now();
                                debug!("START: N/A-{} --- {}", &no, now.format("%H:%M:%S%.f"));

                                job.handle();
                                debug!("FINISH: N/A-{} --- {}", &no, now.format("%H:%M:%S%.f"));
                            }
                        };
                    }
                }

                std::thread::sleep(std::time::Duration::from_millis(100));
            }
        }) {
        Ok(jh) => (Some(jh), Some(tx)),
        Err(e) => {
            error!("Could not start the cron runner thread: {}", e);
            (None, None)
        }
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

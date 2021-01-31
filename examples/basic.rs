use crony::{Job, Runner, Schedule};
use std::thread;
use std::time::Duration;

struct ExampleJob;
impl Job for ExampleJob {
    fn schedule(&self) -> Schedule {
        "1/5 * * * * *".parse().unwrap()
    }
    fn handle(&self) {
        println!("Hello, I am a cron job running at: {}", self.now());
    }
}

fn main() {
    let mut runner = Runner::new();

    println!("Adding ExampleJob to the Runner");
    runner = runner.add(Box::new(ExampleJob));

    println!("Starting the Runner for 20 seconds");
    runner = runner.run();
    thread::sleep(Duration::from_millis(20 * 1000));

    println!("Stopping the Runner");
    runner.stop();
}

# Crony: simple cron runner

Runner that will spawn separate thread where cron jobs will execute.

## Example

```rust
extern crate crony;

use crony::{Job, Runner, Schedule};
use std::str::FromStr;

struct ExampleJob;
impl Job for ExampleJob {
    fn schedule(&self) -> Schedule {
        // Runs every minute
        Schedule::from_str("0 * * * * *").unwrap()
    }
    fn handle(&self) {
        println!("Hello, I am cron job running at: {}", self.now());
    }
}

fn main() {
    println!("Hello world");
    Runner::new().add(Box::new(ExampleJob)).run();
}

/*
Hello world
Hello, I am cron job running at: 2020-12-10 16:01:59.740944 UTC
Hello, I am cron job running at: 2020-12-10 16:02:59.821043 UTC
*/
```

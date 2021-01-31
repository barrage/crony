[![Crates.io](https://img.shields.io/crates/v/crony.svg)](https://crates.io/crates/crony)
[![Docs.rs](https://docs.rs/crony/badge.svg)](https://docs.rs/crony)

# crony

```toml
crony = "0.2.1"
```

## Crony: a simple cron runner

Use the `Job` trait to create your cron job struct, pass it to the `Runner` and then start it via `run()` method.
Runner will spawn new thread where it will start looping through the jobs and will run their handle
method once the scheduled time is reached.

If your OS has enough threads to spare each job will get its own thread to execute, if not it will be
executed in the same thread as the loop but will hold the loop until the job is finished.

Please look at the [**`Job trait`**](./trait.Job.html) documentation for more information.

### Example
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

## License

Licensed under either of

* Apache License, Version 2.0, ([LICENSE-APACHE](LICENSE-APACHE) or https://www.apache.org/licenses/LICENSE-2.0)
* MIT license ([LICENSE-MIT](LICENSE-MIT) or https://opensource.org/licenses/MIT)

at your option.

### Contribution

Unless you explicitly state otherwise, any contribution intentionally
submitted for inclusion in the work by you, as defined in the Apache-2.0
license, shall be dual licensed as above, without any additional terms or
conditions.

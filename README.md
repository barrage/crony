[![Crates.io](https://img.shields.io/crates/v/crony.svg)](https://crates.io/crates/crony)

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
You can run the example using `cargo run --example basic`

```rust
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
```

Output:
```
Adding ExampleJob to the Runner
Starting the Runner for 20 seconds
Hello, I am a cron job running at: 2021-01-31 03:06:25.908475 UTC
Hello, I am a cron job running at: 2021-01-31 03:06:30.912637 UTC
Hello, I am a cron job running at: 2021-01-31 03:06:35.926938 UTC
Hello, I am a cron job running at: 2021-01-31 03:06:40.962138 UTC
Stopping the Runner
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

# rust_supervisor

A Rust library inspired by Erlang/OTP's supervision system, allowing automatic process restart when they fail.

## Overview

`rust_supervisor` brings the robust process supervision model from Erlang/OTP to the Rust ecosystem. It allows you to define processes (threads), monitor their health, and automatically restart them according to configurable strategies when they fail.

## Features

* **Multiple restart strategies**:
  * `OneForOne`: Restart only the failed process
  * `OneForAll`: Restart all processes when one fails
  * `RestForOne`: Restart the failed process and all processes that depend on it

* **Flexible configuration**:
  * Configurable maximum restart attempts
  * Time window for counting restart attempts
  * Process dependency management

* **Process monitoring**:
  * Automatic state tracking (Running, Failed, Restarting, Stopped)
  * Process health monitoring

## Installation

Add `rust_supervisor` to your `Cargo.toml`:

```toml
[dependencies]
rust_supervisor = "0.1.0"
```

## Usage

### Simple Example

```rust
use rust_supervisor::{Supervisor, SupervisorConfig};
use std::thread;
use std::time::Duration;

fn main() {
    // Create a supervisor with default configuration
    let mut supervisor = Supervisor::new(SupervisorConfig::default());
    
    // Add a simple worker process that prints a message every second
    supervisor.add_process("simple_worker", || {
        thread::spawn(|| {
            loop {
                println!("Simple worker is running...");
                
                // Simulate work
                thread::sleep(Duration::from_secs(1));
                
                // Uncomment to simulate random failures
                // if rand::random::<f32>() < 0.1 {
                //     panic!("Worker failed unexpectedly!");
                // }
            }
        })
    });
    
    // Start the supervision
    supervisor.start_monitoring();
    
    println!("Supervisor started. Press Ctrl+C to exit.");
    
    // Keep the main thread alive
    loop {
        thread::sleep(Duration::from_secs(10));
        
        // Check and print the state of our worker
        if let Some(state) = supervisor.get_process_state("simple_worker") {
            println!("Worker state: {:?}", state);
        }
    }
}
```

### Basic Example

```rust
use rust_supervisor::{Supervisor, SupervisorConfig};
use std::thread;
use std::time::Duration;

fn main() {
    // Create a supervisor with default configuration
    let mut supervisor = Supervisor::new(SupervisorConfig::default());
    
    // Add a process to supervise
    supervisor.add_process("worker1", || {
        thread::spawn(|| {
            // Worker code that might fail
            loop {
                println!("Worker 1 running...");
                thread::sleep(Duration::from_secs(1));
                // Simulating a potential failure
                if rand::random::<f32>() < 0.01 {
                    panic!("Worker 1 crashed!");
                }
            }
        })
    });
    
    // Start monitoring the processes
    supervisor.start_monitoring();
    
    // Keep the main thread alive
    loop {
        thread::sleep(Duration::from_secs(1));
    }
}
```

### Dependency Example

```rust
use rust_supervisor::{Supervisor, SupervisorConfig, RestartStrategy};
use std::thread;
use std::time::Duration;

fn main() {
    // Create a supervisor with RestForOne strategy
    let mut config = SupervisorConfig::default();
    config.restart_strategy = RestartStrategy::RestForOne;
    let mut supervisor = Supervisor::new(config);
    
    // Add processes
    supervisor.add_process("database", || {
        thread::spawn(|| {
            // Database connection code...
        })
    });
    
    supervisor.add_process("worker", || {
        thread::spawn(|| {
            // Worker that depends on database...
        })
    });
    
    // Define the dependency
    supervisor.add_dependency("worker", "database");
    
    // Start monitoring
    supervisor.start_monitoring();
}
```

## API Reference

### `SupervisorConfig`

Configuration for the supervisor behavior:

- `max_restarts`: Maximum number of restarts allowed
- `max_time`: Time period over which to count restarts
- `restart_strategy`: Strategy to use when restarting processes

### `Supervisor`

Main supervisor structure:

- `new(config)`: Create a new supervisor
- `add_process(name, factory)`: Add a process to monitor
- `add_dependency(process, depends_on)`: Declare a dependency between processes
- `start_monitoring()`: Start monitoring processes
- `stop_process(name)`: Manually stop a process
- `get_process_state(name)`: Get the current state of a process

### `RestartStrategy`

Defines the strategy to use when a process fails:

- `OneForOne`: Restart only the failed process
- `OneForAll`: Restart all processes when one fails
- `RestForOne`: Restart the failed process and all processes that depend on it

### `ProcessState`

Represents the current state of a process:

- `Running`: Process is running
- `Failed`: Process has failed
- `Restarting`: Process is being restarted
- `Stopped`: Process is stopped (will not be restarted)

## Contributing

Contributions via pull requests are welcome! Feel free to:

- Report bugs
- Suggest new features or enhancements
- Improve documentation
- Submit code improvements

Please ensure your code follows Rust best practices and includes appropriate tests.

## License

This project is licensed under the MIT License - see the LICENSE file for details.

## Acknowledgments

This library is inspired by Erlang/OTP's supervisor behavior, adapting the concept to Rust's threading model.


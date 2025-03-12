use std::thread;
use std::time::Duration;
use rust_supervisor::{Supervisor, SupervisorConfig, RestartStrategy};

fn main() {
    println!("Starting supervision system...");
    
    // Create a supervisor with default configuration
    let mut supervisor = Supervisor::new(SupervisorConfig::default());
    
    // Add a process that will fail periodically
    supervisor.add_process("unstable_process", || {
        thread::spawn(|| {
            println!("Unstable process started");
            
            // Simulate work that eventually fails
            let duration = Duration::from_secs(2);
            thread::sleep(duration);
            
            println!("Unstable process failing!");
            panic!("Simulated error in unstable process");
        })
    });
    
    // Add a stable process that depends on the first one
    supervisor.add_process("stable_process", || {
        thread::spawn(|| {
            println!("Stable process started");
            
            // Infinite loop with periodic logging
            let mut counter = 0;
            loop {
                thread::sleep(Duration::from_secs(1));
                counter += 1;
                println!("Stable process running (iteration {})", counter);
            }
        })
    });
    
    // Declare that the stable process depends on the unstable process
    supervisor.add_dependency("stable_process", "unstable_process");
    
    // Start monitoring
    supervisor.start_monitoring();
    
    println!("Supervision started. Observing activity for 20 seconds...");
    
    // Observe behavior for a certain time
    for i in 1..=20 {
        thread::sleep(Duration::from_secs(1));
        
        // Display process states every 5 seconds
        if i % 5 == 0 {
            if let Some(state) = supervisor.get_process_state("unstable_process") {
                println!("Unstable process state after {} seconds: {:?}", i, state);
            }
            if let Some(state) = supervisor.get_process_state("stable_process") {
                println!("Stable process state after {} seconds: {:?}", i, state);
            }
        }
    }
    
    println!("Demo ended");
}

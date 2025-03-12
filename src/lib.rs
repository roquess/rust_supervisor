//! # rust_supervisor
//! 
//! `rust_supervisor` is a library inspired by Erlang/OTP's supervision system,
//! allowing automatic process restart when they fail.
//!
//! ## Main features
//!
//! * Multiple restart strategies (OneForOne, OneForAll, RestForOne)
//! * Flexible restart policy configuration
//! * Process dependency management
//! * Automatic process state monitoring

use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

/// Defines the strategy to use when a process fails
#[derive(Debug)]
pub enum RestartStrategy {
    /// Restart only the failed process
    OneForOne,
    /// Restart all processes when one fails
    OneForAll,
    /// Restart the failed process and all processes that depend on it
    RestForOne,
}

/// Represents the current state of a process
#[derive(Debug)]
pub enum ProcessState {
    /// Process is running
    Running,
    /// Process has failed
    Failed,
    /// Process is being restarted
    Restarting,
    /// Process is stopped (will not be restarted)
    Stopped,
}

/// Supervisor configuration
#[derive(Debug)]
pub struct SupervisorConfig {
    /// Maximum number of restarts allowed
    pub max_restarts: usize,
    /// Time period over which to count restarts
    pub max_time: Duration,
    /// Restart strategy to use
    pub restart_strategy: RestartStrategy,
}

impl Default for SupervisorConfig {
    /// Creates a default configuration with reasonable values
    fn default() -> Self {
        SupervisorConfig {
            max_restarts: 3,
            max_time: Duration::from_secs(5),
            restart_strategy: RestartStrategy::OneForOne,
        }
    }
}

/// Internal information about a managed process
struct ProcessInfo {
    /// Handle to the running thread (None if not started or failed)
    handle: Option<thread::JoinHandle<()>>,
    /// Restart history for applying the limiting policy
    restart_times: Vec<Instant>,
    /// Current process state
    state: ProcessState,
    /// Factory for creating a new instance of the process
    factory: Box<dyn Fn() -> thread::JoinHandle<()> + Send + 'static>,
}

/// Supervisor that manages a set of processes
pub struct Supervisor {
    /// Map of managed processes, with their name as the key
    processes: Arc<Mutex<HashMap<String, ProcessInfo>>>,
    /// Supervisor configuration
    config: SupervisorConfig,
    /// Map of dependencies between processes
    dependencies: HashMap<String, Vec<String>>,
}

impl Supervisor {
    /// Creates a new supervisor with the specified configuration
    ///
    /// # Arguments
    ///
    /// * `config` - Supervisor configuration
    ///
    /// # Example
    ///
    /// ```
    /// let supervisor = Supervisor::new(SupervisorConfig::default());
    /// ```
    pub fn new(config: SupervisorConfig) -> Self {
        Supervisor {
            processes: Arc::new(Mutex::new(HashMap::new())),
            config,
            dependencies: HashMap::new(),
        }
    }

    /// Adds a process to monitor
    ///
    /// # Arguments
    ///
    /// * `name` - Unique process name
    /// * `factory` - Function that creates and starts the process
    ///
    /// # Example
    ///
    /// ```
    /// supervisor.add_process("worker", || {
    ///     thread::spawn(|| {
    ///         // Worker code...
    ///     })
    /// });
    /// ```
    pub fn add_process<F>(&mut self, name: &str, factory: F)
    where
        F: Fn() -> thread::JoinHandle<()> + Send + 'static,
    {
        let factory_box = Box::new(factory);
        let handle = (factory_box)();
        
        let mut processes = self.processes.lock().unwrap();
        processes.insert(
            name.to_string(),
            ProcessInfo {
                handle: Some(handle),
                restart_times: Vec::new(),
                state: ProcessState::Running,
                factory: factory_box,
            },
        );
    }

    /// Declares a dependency between two processes
    ///
    /// # Arguments
    ///
    /// * `process` - Name of the process that depends on another
    /// * `depends_on` - Name of the process that the first one depends on
    ///
    /// # Example
    ///
    /// ```
    /// // worker2 depends on worker1
    /// supervisor.add_dependency("worker2", "worker1");
    /// ```
    pub fn add_dependency(&mut self, process: &str, depends_on: &str) {
        self.dependencies
            .entry(process.to_string())
            .or_insert_with(Vec::new)
            .push(depends_on.to_string());
    }

    /// Starts monitoring processes
    ///
    /// This method launches a monitoring thread that periodically checks
    /// the state of processes and restarts them according to the configured strategy.
    ///
    /// # Example
    ///
    /// ```
    /// supervisor.start_monitoring();
    /// ```
    pub fn start_monitoring(&self) {
        let processes = Arc::clone(&self.processes);
        let config = self.config.clone();
        let dependencies = self.dependencies.clone();
        
        thread::spawn(move || {
            loop {
                thread::sleep(Duration::from_millis(100));
                
                // First, collect information about failed processes without modifying the map
                let mut failed_processes = Vec::new();
                {
                    let mut processes_lock = processes.lock().unwrap();
                    for (name, info) in processes_lock.iter_mut() {
                        if let Some(handle) = &info.handle {
                            if handle.is_finished() {
                                info.state = ProcessState::Failed;
                                info.handle = None;
                                
                                // Check if we can restart
                                let now = Instant::now();
                                info.restart_times.retain(|time| now.duration_since(*time) < config.max_time);
                                
                                if info.restart_times.len() < config.max_restarts {
                                    failed_processes.push(name.clone());
                                } else {
                                    // Too many restarts, stop the process
                                    info.state = ProcessState::Stopped;
                                }
                            }
                        }
                    }
                }
                
                // Now handle the restart logic for each failed process
                for failed_process in failed_processes {
                    // Determine which processes to restart based on the strategy
                    let processes_to_restart = {
                        let processes_lock = processes.lock().unwrap();
                        match config.restart_strategy {
                            RestartStrategy::OneForOne => vec![failed_process.clone()],
                            RestartStrategy::OneForAll => processes_lock.keys().cloned().collect(),
                            RestartStrategy::RestForOne => {
                                let mut to_restart = vec![failed_process.clone()];
                                // Add processes that depend on this one
                                for (proc_name, deps) in &dependencies {
                                    if deps.contains(&failed_process) {
                                        to_restart.push(proc_name.clone());
                                    }
                                }
                                to_restart
                            }
                        }
                    };
                    
                    // Restart all necessary processes
                    let now = Instant::now();
                    for proc_name in processes_to_restart {
                        let mut processes_lock = processes.lock().unwrap();
                        if let Some(proc_info) = processes_lock.get_mut(&proc_name) {
                            proc_info.state = ProcessState::Restarting;
                            proc_info.handle = Some((proc_info.factory)());
                            proc_info.restart_times.push(now);
                            proc_info.state = ProcessState::Running;
                        }
                    }
                }
            }
        });
    }

    /// Manually stops a process
    ///
    /// # Arguments
    ///
    /// * `name` - Name of the process to stop
    ///
    /// # Returns
    ///
    /// `true` if the process was found and stopped, `false` otherwise
    ///
    /// # Example
    ///
    /// ```
    /// let stopped = supervisor.stop_process("worker1");
    /// ```
    pub fn stop_process(&self, name: &str) -> bool {
        let mut processes = self.processes.lock().unwrap();
        if let Some(info) = processes.get_mut(name) {
            if let Some(handle) = info.handle.take() {
                // In a real implementation, you would want to send a cleaner stop signal
                drop(handle);
                info.state = ProcessState::Stopped;
                return true;
            }
        }
        false
    }

    /// Gets the current state of a process
    ///
    /// # Arguments
    ///
    /// * `name` - Process name
    ///
    /// # Returns
    ///
    /// The process state, or `None` if the process doesn't exist
    ///
    /// # Example
    ///
    /// ```
    /// if let Some(state) = supervisor.get_process_state("worker1") {
    ///     println!("Worker1 state: {:?}", state);
    /// }
    /// ```
    pub fn get_process_state(&self, name: &str) -> Option<ProcessState> {
        let processes = self.processes.lock().unwrap();
        processes.get(name).map(|info| info.state.clone())
    }
}

// Implementation to clone the configuration
impl Clone for SupervisorConfig {
    fn clone(&self) -> Self {
        SupervisorConfig {
            max_restarts: self.max_restarts,
            max_time: self.max_time,
            restart_strategy: match self.restart_strategy {
                RestartStrategy::OneForOne => RestartStrategy::OneForOne,
                RestartStrategy::OneForAll => RestartStrategy::OneForAll,
                RestartStrategy::RestForOne => RestartStrategy::RestForOne,
            },
        }
    }
}

// Clone implementation for RestartStrategy
impl Clone for RestartStrategy {
    fn clone(&self) -> Self {
        match self {
            RestartStrategy::OneForOne => RestartStrategy::OneForOne,
            RestartStrategy::OneForAll => RestartStrategy::OneForAll,
            RestartStrategy::RestForOne => RestartStrategy::RestForOne,
        }
    }
}

// Clone implementation for ProcessState
impl Clone for ProcessState {
    fn clone(&self) -> Self {
        match self {
            ProcessState::Running => ProcessState::Running,
            ProcessState::Failed => ProcessState::Failed,
            ProcessState::Restarting => ProcessState::Restarting,
            ProcessState::Stopped => ProcessState::Stopped,
        }
    }
}

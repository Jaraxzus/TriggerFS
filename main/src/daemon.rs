use std::future::Future;

use crate::signal_handler::cleanup;
use daemonize::Daemonize;
use tokio::runtime::Runtime;
use tracing::info;

pub struct Daemon;

impl Daemon {
    pub fn start<F, Fut>(pid_file: &str, async_main: F) -> Result<(), Box<dyn std::error::Error>>
    where
        F: FnOnce() -> Fut + Send + 'static,
        Fut: Future<Output = ()> + Send + 'static,
    {
        let stdout = std::fs::File::create("/tmp/file-organizer.out")?;
        let stderr = std::fs::File::create("/tmp/file-organizer.err")?;

        let daemonize = Daemonize::new()
            .pid_file(pid_file)
            .chown_pid_file(true)
            .working_directory("/tmp")
            .stdout(stdout)
            .stderr(stderr)
            .privileged_action(|| {
                info!("Privileged action executed.");
            });

        match daemonize.start() {
            Ok(_) => {
                let rt = Runtime::new().expect("Failed to create Tokio runtime");
                rt.block_on(async_main());
            }
            Err(e) => {
                cleanup(pid_file);
                eprintln!("Error while daemonizing: {}", e);
            }
        }

        info!("Daemon started successfully");
        Ok(())
    }
}

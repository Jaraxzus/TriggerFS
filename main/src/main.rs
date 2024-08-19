// main.rs

mod daemon;
mod logger;
mod signal_handler;

use daemon::Daemon;
use tracing::info;

const PID_FILE: &str = "/tmp/file-organizer.pid";

fn main() {
    // Инициализация логгера
    logger::init_logger();

    // Инициализация и запуск демона
    if let Err(e) = Daemon::start(PID_FILE, async_main) {
        eprintln!("Error while daemonizing: {}", e);
    }
}

async fn async_main() {
    let _ = tokio::spawn(run()).await;
    signal_handler::handle_signals(PID_FILE).await;
}

// run основаная работа
async fn run() {
    loop {
        tokio::time::sleep(std::time::Duration::from_secs(3)).await;
        info!("Daemon is still running...");
    }
}

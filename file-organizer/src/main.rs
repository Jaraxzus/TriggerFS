mod daemon;
mod logger;
mod signal_handler;

use daemon::Daemon;
use fs::FsWatcher;

use tracing::error;

const PID_FILE: &str = "/tmp/file-organizer.pid";

fn main() {
    // Инициализация логгера
    logger::init_logger();

    // Инициализация и запуск демона
    if let Err(e) = Daemon::start(PID_FILE, async_main) {
        error!("Error while daemonizing: {}", e);
    }
}

async fn async_main() {
    let _ = tokio::spawn(signal_handler::handle_signals(PID_FILE)).await;
    run().await
}

// run основаная работа
async fn run() {
    let _ = FsWatcher::new().unwrap();
}

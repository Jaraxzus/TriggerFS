mod daemon;
mod logger;
mod signal_handler;

use daemon::Daemon;

use tracing::{error, info};

const PID_FILE: &str = "/tmp/file-organizer.pid";

fn main() {
    // // Инициализация логгера
    logger::init_logger();

    // Инициализация и запуск демона
    if let Err(e) = Daemon::start(PID_FILE, async_main) {
        error!("Error while daemonizing: {}", e);
    }
}

async fn async_main() {
    info!("async_main called");
    tokio::spawn(signal_handler::handle_signals(PID_FILE));
    info!("start actors topology");
    elfo::init::start(main_topology::topology()).await;
}

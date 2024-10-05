use tokio::signal::unix::{signal, SignalKind};
use tracing::{error, info};

// handle_signals слушатель системынх перываний
pub async fn handle_signals(pid_file: &str) {
    let sigterm = async {
        let mut stream = signal(SignalKind::terminate()).expect("Failed to listen for SIGTERM");
        stream.recv().await;
        info!("Received SIGTERM");
    };

    let sigint = async {
        let mut stream = signal(SignalKind::interrupt()).expect("Failed to listen for SIGINT");
        stream.recv().await;
        info!("Received SIGINT");
    };

    tokio::select! {
        _ = sigterm => {
            cleanup(pid_file);
        },
        _ = sigint => {
            cleanup(pid_file);
        },
    }
}

// cleanup синхронный так как ошибка может возникнуть до запуска асинхронного рантайма
pub fn cleanup(pid_file: &str) {
    if let Err(e) = std::fs::remove_file(pid_file) {
        error!("Failed to remove PID file: {}", e);
    } else {
        info!("PID file removed successfully.");
    }
    std::process::exit(0);
}

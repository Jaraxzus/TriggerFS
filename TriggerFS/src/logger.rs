use tracing::Level;
use tracing_subscriber::FmtSubscriber;

pub fn init_logger() {
    let subscriber = FmtSubscriber::builder()
        .with_max_level(Level::TRACE) // Устанавливаем уровень логирования на TRACE
        .finish();

    // tracing::subscriber::set_global_default(subscriber).expect("Failed to set logger");
    // TODO: разобраться с тем как писать логи до демнизации и запуска асинтхронного рантайма
    let _ = tracing::subscriber::set_default(subscriber);
}

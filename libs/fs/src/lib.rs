pub mod actions;
mod fs_watcher;
use std::{
    env,
    path::{Path, PathBuf},
};

pub use fs_watcher::*;

pub fn resolve_path(path: &str) -> PathBuf {
    let path = Path::new(path);

    if path.starts_with("~") {
        // Если путь начинается с '~', заменяем его на путь к домашнему каталогу
        if let Some(home_dir) = dirs::home_dir() {
            // Убираем '~' и соединяем с оставшейся частью пути
            return home_dir.join(path.strip_prefix("~").unwrap());
        } else {
            panic!("Не удалось получить домашний каталог");
        }
    } else if path.is_absolute() {
        // Если путь абсолютный, возвращаем его как есть
        path.to_path_buf()
    } else {
        // Если путь относительный, добавляем текущий рабочий каталог
        let current_dir = env::current_dir().expect("Не удалось получить текущий каталог");
        current_dir.join(path)
    }
}

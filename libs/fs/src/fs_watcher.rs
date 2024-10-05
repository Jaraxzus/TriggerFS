use notify::RecursiveMode;
use notify::{Event, RecommendedWatcher, Watcher};
use serde::Deserialize;
use std::error::Error;
use std::path::Path;
use tokio::sync::mpsc::{channel, Receiver};

#[derive(Debug)]
pub struct FsWatcher {
    pub reciver: Receiver<notify::Result<Event>>,
    watcher: RecommendedWatcher,
}
impl FsWatcher {
    pub fn new() -> Result<FsWatcher, Box<dyn Error>> {
        let (tx, rx) = channel(1);
        let watcher = RecommendedWatcher::new(
            move |res| {
                futures::executor::block_on(async {
                    tx.send(res).await.unwrap();
                })
            },
            notify::Config::default(),
        )?;

        Ok(FsWatcher {
            reciver: rx,
            watcher,
        })
    }

    pub fn async_watch<P: AsRef<Path>>(
        &mut self,
        path: &P,
        mode: &RecursiveModeInernal,
    ) -> Result<(), Box<dyn Error>> {
        self.watcher
            .watch(path.as_ref(), RecursiveMode::from(mode))?;
        Ok(())
    }

    pub async fn unwach<P: AsRef<Path>>(&mut self, path: P) -> Result<(), Box<dyn Error>> {
        self.watcher.unwatch(path.as_ref())?;
        Ok(())
    }
}

#[derive(Debug, Deserialize, Clone)]
#[serde(rename_all = "snake_case")]
pub enum RecursiveModeInernal {
    Recursive,
    NonRecursive,
}

// Конвертация между InternalEnum и ExternalEnum
impl From<&RecursiveModeInernal> for RecursiveMode {
    fn from(item: &RecursiveModeInernal) -> RecursiveMode {
        match item {
            RecursiveModeInernal::Recursive => RecursiveMode::Recursive,
            RecursiveModeInernal::NonRecursive => RecursiveMode::NonRecursive,
        }
    }
}

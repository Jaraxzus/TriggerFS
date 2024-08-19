use notify::RecursiveMode;
use notify::{Event, RecommendedWatcher, Watcher};
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

    // мб добавить защиту от повторного добовления одной и той же дерриктории
    pub async fn async_watch<P: AsRef<Path>>(
        &mut self,
        path: P,
        mode: RecursiveMode,
    ) -> Result<(), Box<dyn Error>> {
        self.watcher.watch(path.as_ref(), mode)?;
        Ok(())
    }

    pub fn unwach<P: AsRef<Path>>(&mut self, path: P) -> Result<(), Box<dyn Error>> {
        self.watcher.unwatch(path.as_ref())?;
        Ok(())
    }
}

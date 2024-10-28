use std::{path::PathBuf, time::Duration};

use elfo::{prelude::*, time::Interval};
use fs::{actions::Action, FsWatcher, RecursiveModeInernal};
use protocol::{FsEvent, KeyAction};
use serde::Deserialize;

use tracing::{error, trace, warn};

pub fn new() -> Blueprint {
    ActorGroup::new()
        .config::<Config>()
        .exec(move |ctx| async move { FsWatcherActor::new(ctx).await.main().await })
}

#[derive(Debug, Deserialize, Clone)]
struct WatcherConf {
    path: PathBuf,
    recursive_mode: RecursiveModeInernal,
    action: Action,
}

#[derive(Debug, Deserialize, Clone)]
struct Config {
    watchers_conf_path: String,
}

struct FsWatcherActor {
    ctx: Context<Config>,
    watchers_conf: Vec<WatcherConf>,
    watcher: FsWatcher,
}

impl FsWatcherActor {
    async fn new(ctx: Context<Config>) -> Self {
        let mut watcher = FsWatcher::new().unwrap_or_else(|err| {
            error!("Encountered an error: {}", err); // Логирование ошибки
            panic!("Aborting due to a critical error: {}", err); // Паника с сообщением
        });
        let config = ctx.config();
        let content = match tokio::fs::read(fs::resolve_path(&config.watchers_conf_path)).await {
            Ok(content) => content,
            Err(err) => {
                error!("Error read WatcherConf: {}", err); // Логирование ошибки
                panic!("Aborting due to a critical error: {}", err); // Паника с сообщением
            }
        };
        let watchers_conf: Vec<WatcherConf> =
            serde_json::from_slice(&content).unwrap_or_else(|err| {
                error!("Error read WatcherConf: {}", err); // Логирование ошибки
                panic!("Aborting due to a critical error: {}", err); // Паника с сообщением
            });

        for path in watchers_conf.iter() {
            if let Err(err) = watcher.async_watch(&path.path, &path.recursive_mode) {
                warn!("for path {:?}: {}", path, err)
            }
        }
        Self {
            ctx,
            watchers_conf,
            watcher,
        }
    }

    async fn main(mut self) {
        loop {
            tokio::select! {
                envelope = self.ctx.recv() => {
                    if let Some(envelope) = envelope {
                        msg!(match envelope {
                            // TODO: должно ли тут быть что то?
                        });
                    }
                }
                event = self.watcher.reciver.recv() => {
                    if let Some(event) = event {
                        match event {
                            Ok(event) => {
                                trace!("give event: {:?}", event);
                                self.process_event(event).await;
                            }
                            Err(e) => error!("watch error: {:?}", e),
                        }
                    }

                }
            }
        }
    }

    async fn process_event(&self, event: notify::Event) {
        trace!("start iteration watchers");
        let mut key_actions = vec![];
        for path in event.paths.iter() {
            for watcher in self.watchers_conf.iter() {
                if path.starts_with(&watcher.path) {
                    key_actions.push(KeyAction {
                        path: path.clone(),
                        action: watcher.action.clone(),
                    });
                }
            }
        }
        self.ctx.send(FsEvent { key_actions, event }).await;
    }
}

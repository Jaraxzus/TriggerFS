use std::path::PathBuf;

use elfo::prelude::*;
use fs::{FsWatcher, RecursiveModeInernal};
use serde::Deserialize;
use tokio::io::{AsyncReadExt, BufReader};

use tracing::{debug, error, info, trace, warn};

pub fn new() -> Blueprint {
    ActorGroup::new()
        .config::<Config>()
        .exec(move |ctx| async move { FsWatcherActror::new(ctx).await.main().await })
}

#[derive(Debug, Deserialize, Clone)]
struct WatcherConf {
    path: PathBuf,
    recursive_mode: RecursiveModeInernal,
    action: fs::actions::Action,
}

#[derive(Debug, Deserialize, Clone)]
struct Config {
    watchers_conf_path: PathBuf,
}

struct FsWatcherActror {
    ctx: Context<Config>,
    watchers_conf: Vec<WatcherConf>,
    watcher: FsWatcher,
}

impl FsWatcherActror {
    async fn new(ctx: Context<Config>) -> Self {
        let mut watcher = FsWatcher::new().unwrap_or_else(|err| {
            error!("Encountered an error: {}", err); // Логирование ошибки
            panic!("Aborting due to a critical error: {}", err); // Паника с сообщением
        });
        let config = ctx.config();
        let content = match tokio::fs::read(&config.watchers_conf_path).await {
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
                                    // More elaborate handling code can be delegated to methods.
                                    // // These methods can easily be async.
                                    // msg @ Increment => self.on_increment(msg),
                                    //
                                    // // Simpler code, however, can still be processed directly here.
                                    // (GetValue, token) => {
                                    //     self.ctx.respond(token, self.value);
                                    // }
                        });

                    }
                                                }
                event = self.watcher.reciver.recv() => {
                    if let Some(event) = event {
                        match event {
                            Ok(event) => {
                                // TODO: пока что вся обработка крутится в одном акторе, мб есть смысл
                                // разделить ее по разным акторам
                                trace!("start iteration watchers");
                                for watcher in self.watchers_conf.iter() {
                                    trace!("start execute action for watcher: {:?}", watcher);
                                    if let Err(err) = watcher.action.execute(&event).await {
                                        error!("{}", err)
                                    }
                                }
                                trace!("event: {:?}", event)
                            }
                            Err(e) => error!("watch error: {:?}", e),
                        }
                    }

                }
            }
        }
    }
}

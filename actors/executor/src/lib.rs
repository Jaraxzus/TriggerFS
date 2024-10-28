use std::time::Duration;

use fs::actions::Action;
use notify::Event;
use serde::Deserialize;

use elfo::{
    prelude::*,
    routers::{MapRouter, Outcome},
    time::Interval,
};

#[message]
struct Tick;

use protocol::*;
use tracing::trace;

pub fn new() -> Blueprint {
    ActorGroup::new()
        .config::<Config>()
        .router(MapRouter::new(|envelope| {
            msg!(match envelope {
                FsEvent { key_actions, .. } => Outcome::Multicast(key_actions.to_vec()),
                _ => Outcome::Default,
            })
        }))
        .exec(move |ctx| async move { ExecutorActor::new(ctx).main().await })
}

#[derive(Debug, Deserialize, Clone)]
struct Config {
    last_event_timeout_ms: u64,
}

// Актор создается на каждый тип Action
struct ExecutorActor {
    ctx: Context<Config, KeyAction>,
    action: Action,
}

impl ExecutorActor {
    fn new(ctx: Context<Config, KeyAction>) -> Self {
        Self {
            action: ctx.key().action.clone(),
            ctx,
        }
    }

    async fn main(mut self) {
        let attached_interval = self.ctx.attach(Interval::new(Tick));

        let mut last_event_time = tokio::time::Instant::now();
        let mut ev = None;
        attached_interval.start(Duration::from_millis(100));
        while let Some(envelope) = self.ctx.recv().await {
            msg!(match envelope {
                FsEvent { event, .. } => {
                    trace!("fs event {:#?}", event);
                    if ev.is_none() {
                        if !self.check_event(&event) {
                            break;
                        }
                        ev = Some(event);
                    }
                    last_event_time = tokio::time::Instant::now();
                }
                Tick => {
                    if tokio::time::Instant::now() - last_event_time
                        >= Duration::from_millis(self.ctx.config().last_event_timeout_ms)
                    {
                        if let Some(ev) = &ev {
                            self.execute_event(ev).await;
                            break;
                        }
                    }
                }
            });
        }
    }
    fn check_event(&self, event: &Event) -> bool {
        self.action.check(event)
    }

    async fn execute_event(&self, event: &Event) {
        self.action.execute(event).await;
    }
}

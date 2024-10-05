use fs::actions::Action;
use notify::Event;
use serde::Deserialize;

use elfo::{
    prelude::*,
    routers::{MapRouter, Outcome},
};

use protocol::*;

pub fn new() -> Blueprint {
    ActorGroup::new()
        .config::<Config>()
        .router(MapRouter::new(|envelope| {
            msg!(match envelope {
                FsEvent { key_actions, .. } =>
                // Outcome::Multicast(key_actions.iter().map(|k_a| k_a.0.clone()).collect()),
                    Outcome::Multicast(key_actions.to_vec()),
                _ => Outcome::Default,
            })
        }))
        .exec(move |ctx| async move { ExecutorActor::new(ctx).main().await })
}

#[derive(Debug, Deserialize, Clone)]
struct Config {
    #[serde(default)]
    todo: Option<String>,
}

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
        while let Some(envelope) = self.ctx.recv().await {
            msg!(match envelope {
                FsEvent { event, .. } => self.process_event(event).await,
            });
        }
    }
    async fn process_event(&self, event: Event) {
        self.action.execute(&event).await;
    }
}

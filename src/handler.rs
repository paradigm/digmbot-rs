use crate::{
    config::Config, context::Context, event::Event, persistent_state::PersistentState,
    volatile_state::VolatileState,
};
use serenity::all::{Message, Reaction, Ready, VoiceState};
use tokio::sync::RwLock;

/// Discord event handler
pub struct Handler {
    cfg: RwLock<Config>,
    pstate: RwLock<PersistentState>,
    vstate: RwLock<VolatileState>,
}

impl<'a> Handler {
    pub fn new(cfg: Config, pstate: PersistentState, vstate: VolatileState) -> Self {
        Self {
            cfg: RwLock::new(cfg),
            pstate: RwLock::new(pstate),
            vstate: RwLock::new(vstate),
        }
    }

    fn ctx(&'a self, discord_ctx: &'a serenity::all::Context) -> Context<'a> {
        Context {
            cfg: &self.cfg,
            pstate: &self.pstate,
            vstate: &self.vstate,
            cache: &discord_ctx.cache,
            http: &discord_ctx.http,
            cache_http: discord_ctx,
        }
    }
}

#[serenity::async_trait]
impl serenity::all::EventHandler for Handler {
    async fn ready(&self, discord_ctx: serenity::all::Context, ready: Ready) {
        Event::Ready(ready).handle(self.ctx(&discord_ctx)).await;
    }

    async fn message(&self, discord_ctx: serenity::all::Context, msg: Message) {
        Event::Message(msg).handle(self.ctx(&discord_ctx)).await;
    }

    async fn voice_state_update(
        &self,
        discord_ctx: serenity::all::Context,
        old: Option<VoiceState>,
        new: VoiceState,
    ) {
        Event::VoiceStateUpdate { old, new }
            .handle(self.ctx(&discord_ctx))
            .await;
    }

    async fn reaction_add(&self, discord_ctx: serenity::all::Context, reaction: Reaction) {
        Event::ReactionAdd(reaction)
            .handle(self.ctx(&discord_ctx))
            .await;
    }

    async fn reaction_remove(&self, discord_ctx: serenity::all::Context, reaction: Reaction) {
        Event::ReactionRemove(reaction)
            .handle(self.ctx(&discord_ctx))
            .await;
    }
}

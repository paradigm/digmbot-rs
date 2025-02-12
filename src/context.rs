use crate::{config::Config, persistent_state::PersistentState, volatile_state::VolatileState};
use std::sync::Arc;
use tokio::sync::RwLock;

/// Collection of data that is shared across events
pub struct Context<'a> {
    // Digmbot's own context types
    pub cfg: &'a RwLock<Config>,
    pub pstate: &'a RwLock<PersistentState>,
    pub vstate: &'a RwLock<VolatileState>,
    // Discord/Serenity context types
    pub cache: &'a Arc<serenity::all::Cache>,
    pub http: &'a Arc<serenity::all::Http>,
    pub cache_http: &'a CacheHttp,
}

/// Many Serenity functions take a `impl CacheHttp` in order to first check the cache if the item
/// is available and fall back to an http request otherwise.  The most readily available type that
/// impl's this is named very differently in a way that could be confusing, and so we alias it.
pub type CacheHttp = serenity::all::Context;

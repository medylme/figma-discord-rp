use std::sync::OnceLock;

use crate::VERSION;

pub mod core;
pub mod download;
pub mod install;
pub mod splash;

fn user_agent() -> &'static str {
    static UA: OnceLock<String> = OnceLock::new();
    UA.get_or_init(|| format!("figma-discord-rp/{}", VERSION))
}

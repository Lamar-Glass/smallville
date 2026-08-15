use std::env;

use clap::Parser;

#[derive(Parser, Debug)]
#[command(
    name = "smallville",
    version,
    about = "Run the Smallville generative-agents simulation and watch AI agents build a civilization",
    long_about = "Ten LLM-driven agents live out their days in a scripted town with memory streams, \
    reflections, and daily plans. Watch the town come alive in the browser.\n\n\
    Requires OPENAI_API_KEY (any OpenAI-compatible API). Set SIM_MOCK=1 to run \
    a fully scripted town with no API calls."
)]
pub struct Args {
    #[arg(long, default_value_t = 8080)]
    pub port: u16,

    #[arg(long, default_value_t = 8, help = "real seconds per in-game hour")]
    pub hour_seconds: u64,

    #[arg(long, help = "chat model (default $SIM_MODEL or gpt-4o-mini)")]
    pub model: Option<String>,

    #[arg(long, help = "embedding model (default $SIM_EMBED_MODEL or text-embedding-3-small)")]
    pub embed_model: Option<String>,

    #[arg(long, help = "OpenAI-compatible base URL (default $SIM_BASE_URL or https://api.openai.com/v1)")]
    pub base_url: Option<String>,
}

#[derive(Clone, Debug)]
pub struct Config {
    pub port: u16,
    pub hour_seconds: u64,
    pub model: String,
    pub embed_model: String,
    pub base_url: String,
}

impl Config {
    pub fn from_args(args: &Args) -> Self {
        Self {
            port: args.port,
            hour_seconds: args.hour_seconds,
            model: args
                .model
                .clone()
                .or_else(|| env::var("SIM_MODEL").ok())
                .unwrap_or_else(|| "gpt-4o-mini".to_string()),
            embed_model: args
                .embed_model
                .clone()
                .or_else(|| env::var("SIM_EMBED_MODEL").ok())
                .unwrap_or_else(|| "text-embedding-3-small".to_string()),
            base_url: args
                .base_url
                .clone()
                .or_else(|| env::var("SIM_BASE_URL").ok())
                .unwrap_or_else(|| "https://api.openai.com/v1".to_string()),
        }
    }
}

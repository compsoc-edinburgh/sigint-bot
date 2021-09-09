//! Requires the 'framework' feature flag be enabled in your project's
//! `Cargo.toml`.
//!
//! This can be enabled by specifying the feature in the dependency section:
//!
//! ```toml
//! [dependencies.serenity]
//! git = "https://github.com/serenity-rs/serenity.git"
//! features = ["framework", "standard_framework"]
//! ```
mod commands;

use std::{collections::HashSet, fs::read_to_string, sync::Arc};

use commands::welcome::*;
use serde::Deserialize;
use serenity::{
    async_trait,
    client::bridge::gateway::ShardManager,
    framework::{standard::macros::group, StandardFramework},
    http::Http,
    model::prelude::*,
    prelude::*,
};
use tracing::{error, info};
use tracing_subscriber::{EnvFilter, FmtSubscriber};

pub(crate) struct ShardManagerContainer;
pub(crate) struct ConfigContainer;

impl TypeMapKey for ShardManagerContainer {
    type Value = Arc<Mutex<ShardManager>>;
}

impl TypeMapKey for ConfigContainer {
    type Value = Config;
}

struct Handler;

#[async_trait]
impl EventHandler for Handler {
    async fn ready(&self, _: Context, ready: Ready) {
        info!("Connected as {}", ready.user.name);
    }

    async fn resume(&self, _: Context, _: ResumedEvent) {
        info!("Resumed");
    }
}

#[derive(Deserialize)]
pub(crate) struct Config {
    discord_token: String,
    guild_id: u64,
    welcome: WelcomeConfig,
}

#[derive(Deserialize)]
pub(crate) struct WelcomeConfig {
    flag: String,
    role_id: u64,
}

#[group]
#[commands(welcome)]
#[only_in(dm)]
struct General;

#[tokio::main]
async fn main() {
    // Load configurations.
    let config: Config =
        toml::from_str(&read_to_string("config.toml").expect("Error accessing config.toml"))
            .expect("Error parsing config.toml");

    // Initialize the logger to use environment variables.
    let subscriber = FmtSubscriber::builder()
        .with_env_filter(EnvFilter::from_default_env())
        .finish();

    tracing::subscriber::set_global_default(subscriber).expect("Failed to start the logger");

    let http = Http::new_with_token(&config.discord_token);

    // We will fetch your bot's owners and id
    let (owners, _bot_id) = match http.get_current_application_info().await {
        Ok(info) => {
            let mut owners = HashSet::new();
            owners.insert(info.owner.id);

            (owners, info.id)
        }
        Err(why) => panic!("Could not access application info: {:?}", why),
    };

    // Create the framework
    let framework = StandardFramework::new()
        .configure(|c| c.owners(owners).prefix(":"))
        .group(&GENERAL_GROUP);

    let mut client = Client::builder(&config.discord_token)
        .framework(framework)
        .event_handler(Handler)
        .await
        .expect("Error creating client");

    {
        let mut data = client.data.write().await;
        data.insert::<ShardManagerContainer>(client.shard_manager.clone());
        data.insert::<ConfigContainer>(config);
    }

    let shard_manager = client.shard_manager.clone();

    tokio::spawn(async move {
        tokio::signal::ctrl_c()
            .await
            .expect("Could not register ctrl+c handler");
        shard_manager.lock().await.shutdown_all().await;
    });

    if let Err(why) = client.start().await {
        error!("Client error: {:?}", why);
    }
}

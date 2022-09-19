#![warn(clippy::all)]
#![warn(clippy::pedantic)]
#![warn(clippy::nursery)]
#![allow(clippy::no_effect_underscore_binding)]

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

use commands::ctftime::get_upcoming_ctf;
use commands::{register_commands::register_slash_commands, welcome};
use poise::{serenity_prelude as serenity, Framework, PrefixFrameworkOptions};

use serenity::{Client, EventHandler, Mutex, TypeMapKey};
use std::{collections::HashSet, fs::read_to_string, sync::Arc};
use tracing::callsite::register;

use poise::serenity_prelude::SerenityError;
use serde::Deserialize;
use serenity::model::prelude::{Ready, ResumedEvent};
use serenity::{async_trait, http::Http};
use tracing::{error, info};
use tracing_subscriber::{EnvFilter, FmtSubscriber};

use crate::welcome::welcome;

type Context<'a> = poise::Context<'a, Data, SerenityError>;

pub(crate) struct ShardManagerContainer;
pub(crate) struct ConfigContainer;

impl TypeMapKey for ShardManagerContainer {
    type Value = Arc<Mutex<serenity::ShardManager>>;
}

impl TypeMapKey for ConfigContainer {
    type Value = Config;
}

pub struct Data {}

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

    // Create the framework
    let client = Framework::builder()
        .options(poise::FrameworkOptions {
            // TODO: Add allowed mentions
            commands: vec![welcome(), register_slash_commands(), get_upcoming_ctf()],
            prefix_options: PrefixFrameworkOptions {
                prefix: Some("!".to_string()),
                additional_prefixes: Vec::new(),
                mention_as_prefix: true,
                ignore_bots: true,
                case_insensitive_commands: true,
                ..Default::default()
            },
            ..Default::default()
        })
        .token(&config.discord_token)
        .intents(
            serenity::GatewayIntents::non_privileged() | serenity::GatewayIntents::MESSAGE_CONTENT,
        )
        .user_data_setup(move |_ctx, _ready, _framework| Box::pin(async move { Ok(Data {}) }))
        .build()
        .await
        .expect("Error creating client");

    {
        let client_data = client.client();
        let mut data = client_data.data.write().await;
        data.insert::<ShardManagerContainer>(client.shard_manager().clone());
        data.insert::<ConfigContainer>(config);
    }

    let shard_manager = client.shard_manager().clone();

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

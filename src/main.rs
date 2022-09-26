#![warn(clippy::all)]
#![warn(clippy::pedantic)]
#![warn(clippy::nursery)]
#![allow(clippy::no_effect_underscore_binding)]

mod commands;

use chrono::Utc;
use commands::{
    ctftime::{generate_embed, get_upcoming_ctf, Ctf, TimeFrame},
    register_commands::register_slash_commands,
    welcome,
};
use poise::{
    serenity_prelude::{self as serenity, ChannelId, SerenityError},
    Framework, PrefixFrameworkOptions,
};
use serde::Deserialize;
use serenity::{Mutex, TypeMapKey};
use std::{collections::HashSet, fs::read_to_string, sync::Arc};
use tracing::info;
use tracing_subscriber::{EnvFilter, FmtSubscriber};

use crate::commands::ctftime::assign_ctf_announcement_role;
use crate::welcome::welcome;

type Context<'a> = poise::Context<'a, Arc<Mutex<Data>>, SerenityError>;

pub(crate) struct ShardManagerContainer;
pub(crate) struct ConfigContainer;

impl TypeMapKey for ShardManagerContainer {
    type Value = Arc<Mutex<serenity::ShardManager>>;
}

impl TypeMapKey for ConfigContainer {
    type Value = Arc<Config>;
}

#[derive(Eq, Hash, PartialEq)]
pub struct CTFLog {
    pub ctf_id: usize,
    pub finish: chrono::DateTime<Utc>,
}

pub struct Data {
    pub previously_shown: HashSet<CTFLog>,
}

impl Data {
    #[must_use]
    pub fn new() -> Self {
        Self {
            previously_shown: HashSet::new(),
        }
    }
}

impl Default for Data {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Deserialize, Clone)]
pub(crate) struct Config {
    discord_token: String,
    guild_id: u64,
    welcome: WelcomeConfig,
    notification_channel_id: u64,
    notification_role_id: u64,
    ctftime_loop_seconds: u64,
}

#[derive(Deserialize, Clone)]
pub(crate) struct WelcomeConfig {
    flag: String,
    role_id: u64,
}

#[tokio::main]
async fn main() {
    // Load configurations.
    let config: Arc<Config> = Arc::new(
        toml::from_str(&read_to_string("config.toml").expect("Error accessing config.toml"))
            .expect("Error parsing config.toml"),
    );

    // Initialize the logger to use environment variables.
    let subscriber = FmtSubscriber::builder()
        .with_env_filter(EnvFilter::from_default_env())
        .finish();

    tracing::subscriber::set_global_default(subscriber).expect("Failed to start the logger");

    let config_clone = config.clone();

    // Create the framework
    let client = Framework::builder()
        .options(poise::FrameworkOptions {
            // TODO: Add allowed mentions
            commands: vec![
                welcome(),
                register_slash_commands(),
                get_upcoming_ctf(),
                assign_ctf_announcement_role(),
            ],
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
        .user_data_setup(move |ctx, _ready, _framework| {
            Box::pin(async move {
                let user_data = Arc::new(Mutex::new(Data::new()));
                let user_data_clone = user_data.clone();

                post_ctf_loop(user_data_clone, config_clone, ctx.clone());
                Ok(user_data)
            })
        })
        .build()
        .await
        .expect("Error creating client");

    {
        let client_data = client.client();
        let mut data = client_data.data.write().await;
        data.insert::<ShardManagerContainer>(client.shard_manager().clone());
        data.insert::<ConfigContainer>(config.clone());
    }

    info!("Client Created");

    let shard_manager = client.shard_manager().clone();

    tokio::spawn(async move {
        tokio::signal::ctrl_c()
            .await
            .expect("Could not register ctrl+c handler");
        shard_manager.lock().await.shutdown_all().await;
    });

    info!("Shard manager created");

    if let Err(e) = client.start().await {
        tracing::error!("Client error: {:?}", e);
    }
}

fn post_ctf_loop(
    user_data: Arc<Mutex<Data>>,
    config: Arc<Config>,
    ctx: poise::serenity_prelude::Context,
) {
    // Loop to update us with upcoming ctfs. Also keeps a log of all previously displayed CTFS to make sure we don't display them multiple times.
    // Clear all ctfs in the past to stop memory leaks. This state is used to make sure we don't show multiple ctfs
    tokio::spawn(async move {
        info!("Creating Interval");
        let mut interval =
            tokio::time::interval(std::time::Duration::from_secs(config.ctftime_loop_seconds));

        info!("Interval created");
        loop {
            info!("Started checker loop");
            interval.tick().await;

            // Load all ctfs
            let ctfs = Ctf::get_ctfs(TimeFrame::Week.to_duration()).await.unwrap();

            // Remove all old saved ctfs that are now finished.
            let mut user_data_locked = user_data.lock().await;
            user_data_locked
                .previously_shown
                .retain(|ctf| ctf.finish > chrono::Utc::now());

            // TODO: This is inefficient, please fix
            let mut unseen = ctfs
                .iter()
                .filter(|x| {
                    !user_data_locked
                        .previously_shown
                        .contains(&CTFLog::from((*x).clone()))
                })
                .collect::<Vec<_>>();

            // Add all new entries into the hashmap so we don't post them again
            for &x in &unseen {
                user_data_locked.previously_shown.insert(x.clone().into());
            }

            // Drop the lock as we will be doing network requests
            drop(user_data_locked);

            if !unseen.is_empty() {
                // Post each new ctf into the channel
                let channel = ChannelId(config.notification_channel_id)
                    .to_channel(ctx.http.clone())
                    .await
                    .unwrap();

                channel
                    .id()
                    .send_message(ctx.http.clone(), |b| {
                        b.allowed_mentions(|am| {
                            am.empty_parse().roles(vec![config.notification_role_id])
                        })
                        .content(format!("<@&{}>", config.notification_role_id))
                    })
                    .await
                    .unwrap();

                //
                unseen.sort_unstable_by_key(|x| x.finish());

                for ctf in unseen {
                    channel
                        .id()
                        .send_message(ctx.http.clone(), |b| {
                            b.add_embed(|eb| generate_embed(ctf, eb))
                        })
                        .await
                        .unwrap();
                }
            }
        }
    });
}

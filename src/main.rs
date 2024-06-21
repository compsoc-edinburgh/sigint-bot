#![warn(clippy::all)]
#![warn(clippy::pedantic)]
#![warn(clippy::nursery)]
#![allow(clippy::no_effect_underscore_binding)]

mod commands;

use chrono::Utc;
use commands::{
    // ctfnote::ctfnote_link,
    ctftime::{generate_embed, get_upcoming_ctf, Ctf, TimeFrame},
    // register_commands::register_slash_commands,
    // welcome,
};
use poise::{
    serenity_prelude::{
        self as serenity, futures::lock::Mutex, prelude::TypeMapKey, ChannelId, ClientBuilder,
        CreateAllowedMentions, Error,
    },
    Framework, PrefixFrameworkOptions,
};
use serde::Deserialize;
use serenity::builder::CreateMessage;
use std::{collections::HashSet, fs::read_to_string, sync::Arc};
use tracing::{error, info, log::warn};
use tracing_subscriber::{EnvFilter, FmtSubscriber};

// use crate::commands::ctftime::assign_ctf_announcement_role;
// use crate::welcome::welcome;

type Context<'a> = poise::Context<'a, Data, Error>;

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

pub struct PostCtfLoopData {
    pub previously_shown: HashSet<CTFLog>,
}

impl PostCtfLoopData {
    #[must_use]
    pub fn new() -> Self {
        Self {
            previously_shown: HashSet::new(),
        }
    }
}

impl Default for PostCtfLoopData {
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
    ctfnote_admin_api_endpoint: String,
    ctfnote_admin_api_password: String,
}

#[derive(Deserialize, Clone)]
pub(crate) struct WelcomeConfig {
    flag: String,
    role_id: u64,
}

// Custom user data passed to all command functions
pub struct Data {}

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

    let config_clone = config.clone();

    // Create the framework
    let framework = Framework::builder()
        .options(poise::FrameworkOptions {
            // TODO: Add allowed mentions
            commands: vec![
                // welcome(),
                // register_slash_commands(),
                get_upcoming_ctf(),
                // assign_ctf_announcement_role(),
                // ctfnote_link(),
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
        .setup(move |ctx, _ready, _framework| {
            Box::pin(async move {
                post_ctf_loop(config_clone, ctx.clone());
                Ok(Data {})
            })
        })
        .build();

    let token = &config.discord_token;
    let intents =
        serenity::GatewayIntents::non_privileged() | serenity::GatewayIntents::MESSAGE_CONTENT;
    let client = ClientBuilder::new(token, intents)
        .framework(framework)
        .await;

    // {
    //     let client_data = client.client();
    //     let mut data = client_data.data.write().await;
    //     data.insert::<ShardManagerContainer>(client.shard_manager().clone());
    //     data.insert::<ConfigContainer>(config.clone());
    // }

    // info!("Client Created");

    // let shard_manager = client.shard_manager().clone();

    // tokio::spawn(async move {
    //     tokio::signal::ctrl_c()
    //         .await
    //         .expect("Could not register ctrl+c handler");
    //     shard_manager.lock().await.shutdown_all().await;
    // });

    info!("Shard manager created");

    if let Err(e) = client.unwrap().start().await {
        tracing::error!("Client error: {:?}", e);
    }
}

fn post_ctf_loop(config: Config, ctx: poise::serenity_prelude::Context) {
    // Loop to update us with upcoming ctfs. Also keeps a log of all previously displayed CTFS to make sure we don't display them multiple times.
    // Clear all ctfs in the past to stop memory leaks. This state is used to make sure we don't show multiple ctfs
    let user_data = Arc::new(Mutex::new(PostCtfLoopData::new()));
    tokio::spawn(async move {
        info!("Creating Interval");
        let mut interval =
            tokio::time::interval(std::time::Duration::from_secs(config.ctftime_loop_seconds));

        info!("Interval created");
        loop {
            info!("Started checker loop");
            interval.tick().await;

            // Load all ctfs
            let ctfs = match Ctf::get_ctfs(TimeFrame::Week.to_duration()).await {
                Ok(x) => x,
                Err(err) => {
                    error!("Failed to get ctfs: {:?}", err);
                    break;
                }
            };

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
                let channel = ChannelId::new(config.notification_channel_id)
                    .to_channel(ctx.http.clone())
                    .await
                    .unwrap();

                channel
                    .id()
                    .send_message(
                        ctx.http.clone(),
                        CreateMessage::new()
                            .allowed_mentions(
                                CreateAllowedMentions::new()
                                    .roles(vec![config.notification_role_id]),
                            )
                            .content(format!("<@&{}>", config.notification_role_id)),
                    )
                    .await
                    .unwrap();

                //
                unseen.sort_unstable_by_key(|x| x.finish());

                for ctf in unseen {
                    channel
                        .id()
                        .send_message(
                            ctx.http.clone(),
                            CreateMessage::new().add_embed(generate_embed(ctf)),
                        )
                        .await
                        .unwrap();
                }
            }
        }
    });
}

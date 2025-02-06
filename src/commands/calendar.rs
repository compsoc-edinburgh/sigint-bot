use std::vec;

use chrono::{serde::ts_seconds, DateTime, Utc};
use poise::{
    serenity_prelude::{
        ChannelId, ComponentInteractionCollector, CreateActionRow, CreateAllowedMentions, CreateButton, CreateEmbed, CreateInteractionResponse, CreateInteractionResponseMessage, CreateMessage, Error
    },
    CreateReply,
};
use serde::{Deserialize, Serialize};
use slug::slugify;
use tracing::info;

use crate::{Config, Context};

fn calendar_reminder_loop(config: Config, ctx: poise::serenity_prelude::Context) {
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

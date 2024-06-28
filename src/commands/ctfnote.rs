use std::vec;

use chrono::{serde::ts_seconds, DateTime, Utc};
use poise::{
    serenity_prelude::{
        ComponentInteractionCollector, CreateActionRow, CreateButton, CreateEmbed, CreateInteractionResponse, CreateInteractionResponseMessage, Error
    },
    CreateReply,
};
use serde::{Deserialize, Serialize};

use crate::Context;

#[derive(Serialize)]
struct CtfnoteLinkRequest {
    token: String,
    discord_id: String,
}

#[derive(Deserialize)]
struct CtfnoteLinkResponse {
    message: String,
}

/// Connect your Discord account to your CTFNote account!
#[poise::command(slash_command)]
pub async fn ctfnote_link(
    ctx: Context<'_>,
    #[description = "Your CTFNote account token (found in your profile)"] token: String,
) -> Result<(), Error> {
    let config = &ctx.data().config;
    let ctfnote_extra_url = &config.ctfnote.ctfnote_extra_url;
    let ctfnote_admin_api_password = &config.ctfnote.ctfnote_admin_api_password;

    let discord_id = ctx.author().id;
    let client = reqwest::Client::new();
    let res = client
        .post(format!("{}/api/admin/link-discord", ctfnote_extra_url))
        .basic_auth("admin", Some(ctfnote_admin_api_password))
        .json(&CtfnoteLinkRequest {
            token: token.to_string(),
            discord_id: discord_id.to_string(),
        })
        .send()
        .await?;
    let response = res.json::<CtfnoteLinkResponse>().await?;

    ctx.send(
        CreateReply::default()
            .ephemeral(true)
            .content(format!("{}", response.message)),
    )
    .await?;
    Ok(())
}

#[derive(Serialize)]
struct GetTokenForDiscordUserRequest {
    discord_id: String,
}

#[derive(Deserialize)]
struct GetTokenForDiscordUserResponse {
    token: Option<Token>,
    message: String,
}

#[derive(Deserialize)]
#[allow(dead_code)]
pub struct Token {
    token: String,
    pub user_id: i32,
    exp: i64,
}

#[derive(Deserialize)]
#[allow(dead_code)]
struct JwtClaim {
    user_id: i32,
    role: String,
    exp: usize,
    iat: usize,
    aud: String,
    iss: String,
}

/// Login to CTFNote account.
#[poise::command(slash_command)]
pub async fn ctfnote_login(ctx: Context<'_>) -> Result<(), Error> {
    let config = &ctx.data().config;
    let ctfnote_extra_url = &config.ctfnote.ctfnote_extra_url;
    let ctfnote_admin_api_password = &config.ctfnote.ctfnote_admin_api_password;

    let discord_id = ctx.author().id;
    let client = reqwest::Client::new();
    let token_endpoint = format!("{}/api/admin/get-token", ctfnote_extra_url);
    let res = client
        .post(&token_endpoint)
        .basic_auth("admin", Some(ctfnote_admin_api_password))
        .json(&GetTokenForDiscordUserRequest {
            discord_id: discord_id.to_string(),
        })
        .send()
        .await?;
    let response = res.json::<GetTokenForDiscordUserResponse>().await?;
    let token = response.token;
    match token {
        Some(token) => {
            ctx.send(CreateReply::default().ephemeral(true).content(format!(
                "<{}/token-login?token={}>\nExpires <t:{}>",
                ctfnote_extra_url, token.token, token.exp
            )))
            .await?;
        }
        None => {
            ctx.send(
                CreateReply::default()
                    .ephemeral(true)
                    .content(format!("{}", response.message)),
            )
            .await?;
        }
    }

    Ok(())
}

#[derive(Serialize)]
struct CtfnoteRegisterRequest {
    username: String,
    discord_id: String,
}

#[derive(Deserialize)]
struct CtfnoteRegisterResponse {
    message: String,
}

/// Create CTFNote account
#[poise::command(slash_command, guild_only)]
pub async fn ctfnote_create_account(
    ctx: Context<'_>,
    #[description = "Username"] username: Option<String>,
) -> Result<(), Error> {
    let config = &ctx.data().config;

    // only runs in team channel
    let team_channel_id = config.team_channel_id;
    let channel_id = ctx.channel_id().get();
    if channel_id != team_channel_id {
        ctx.reply(format!(
            "You can only run this command in team channel <#{}>",
            team_channel_id
        ))
        .await?;
        return Ok(());
    }

    let client = reqwest::Client::new();
    let author = ctx.author();
    let username = match username {
        Some(username) => username,
        None => author.name.clone(),
    };
    let discord_id = author.id;
    let res = client
        .post(format!(
            "{}/api/admin/register",
            &config.ctfnote.ctfnote_extra_url
        ))
        .basic_auth("admin", Some(&config.ctfnote.ctfnote_admin_api_password))
        .json(&CtfnoteRegisterRequest {
            username,
            discord_id: discord_id.to_string(),
        })
        .send()
        .await?;
    let response = res.json::<CtfnoteRegisterResponse>().await?;
    ctx.reply(format!("{}", response.message)).await?;

    Ok(())
}

#[derive(Serialize)]
#[allow(dead_code)]
struct GetRoleForDiscordUserRequest {
    discord_id: String,
}

#[derive(Deserialize)]
#[allow(dead_code)]
struct GetRoleForDiscordUserResponse {
    role: Option<String>,
    message: String,
}

#[derive(Deserialize, Debug)]
#[allow(dead_code)]
struct Ctf {
    id: i32,
    title: String,
    weight: f64,
    ctf_url: String,
    logo_url: String,
    ctftime_url: String,
    description: String,
    #[serde(with = "ts_seconds")]
    start_time: DateTime<Utc>,
    #[serde(with = "ts_seconds")]
    end_time: DateTime<Utc>,
    // secrets_id: foreign key
}

#[derive(Deserialize)]
struct GetUpcomingCtfResponse(Vec<Ctf>);

#[derive(Serialize)]
struct AddDiscordUserToCtfRequest {
    discord_id: String,
    ctf_id: i32,
}

#[derive(Deserialize)]
struct AddDiscordUserToCtfResponse {
    message: String,
}

/// Announce upcoming CTFs on CTFNote in the channel
#[poise::command(slash_command, guild_only)]
pub async fn ctfnote_announce_upcoming(ctx: Context<'_>) -> Result<(), Error> {
    let config = &ctx.data().config;

    // only CTFNote manager or admin can announce
    let discord_id = ctx.author().id;
    let client = reqwest::Client::new();
    let res = client
        .get(format!(
            "{}/api/admin/role",
            &config.ctfnote.ctfnote_extra_url
        ))
        .basic_auth("admin", Some(&config.ctfnote.ctfnote_admin_api_password))
        .query(&GetRoleForDiscordUserRequest {
            discord_id: discord_id.to_string(),
        })
        .send()
        .await?;
    let response = res.json::<GetRoleForDiscordUserResponse>().await?;
    let role = response.role;
    let role_string = role.unwrap_or("".to_string());
    if role_string != "user_manager" && role_string != "user_admin" {
        ctx.reply(
            "You need to be CTFNote manager or admin, or your Discord is not linked to CTFNote",
        )
        .await?;
        return Ok(());
    }

    let res = client
        .get(format!(
            "{}/api/admin/upcoming-ctf",
            &config.ctfnote.ctfnote_extra_url
        ))
        .basic_auth("admin", Some(&config.ctfnote.ctfnote_admin_api_password))
        .send()
        .await?;
    let response = res.json::<GetUpcomingCtfResponse>().await?;

    if response.0.len() == 0 {
        ctx.reply("No upcoming CTFs on CTFNote.").await?;
        return Ok(());
    }

    for ctf in response.0 {
        let custom_id = format!("ctfnote_join_ctf:{}", ctf.id);
        let reply = ctx.send(
            CreateReply::default()
                .embed(
                    CreateEmbed::new()
                        .title(&ctf.title)
                        .description(&ctf.description)
                        .thumbnail(&ctf.logo_url)
                        .field(
                            "Dates",
                            format!(
                                "Starts: <t:{}:f>.\n Ends: <t:{}:f>",
                                ctf.start_time.timestamp(),
                                ctf.end_time.timestamp(),
                            ),
                            true,
                        )
                        .field("CTF Page", &ctf.ctf_url, true)
                        .url(&ctf.ctftime_url)
                        .fields([("Weight", &ctf.weight.to_string(), true)]),
                )
                .components(vec![CreateActionRow::Buttons(vec![CreateButton::new(
                    custom_id.clone(),
                )
                .label("Join on CTFNote")])]),
        )
        .await?;

        let mut custom_id_ = custom_id.clone();
        while let Some(mci) = ComponentInteractionCollector::new(ctx)
            .message_id(reply.message().await.unwrap().id)
            .guild_id(ctx.guild_id().unwrap())
            .channel_id(ctx.channel_id())
            .timeout(std::time::Duration::from_secs(30 * 24 * 60 * 60)) // 30 days
            .filter(move |mci| mci.data.custom_id == custom_id_.clone())
            .await
        {
            let res = client
                .post(format!(
                    "{}/api/admin/add-to-ctf",
                    &config.ctfnote.ctfnote_extra_url
                ))
                .basic_auth("admin", Some(&config.ctfnote.ctfnote_admin_api_password))
                .json(&AddDiscordUserToCtfRequest {
                    discord_id: mci.user.id.to_string(),
                    ctf_id: ctf.id

                })
                .send()
                .await?;
            let response = res.json::<AddDiscordUserToCtfResponse>().await?;
            mci.create_response(ctx, CreateInteractionResponse::Message(CreateInteractionResponseMessage::default().content(response.message))).await?;

            custom_id_ = custom_id.clone();
        }
    }

    Ok(())
}

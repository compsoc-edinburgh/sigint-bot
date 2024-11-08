use std::vec;

use chrono::{serde::ts_seconds, DateTime, Utc};
use poise::{
    serenity_prelude::{
        ComponentInteractionCollector, CreateActionRow, CreateAllowedMentions, CreateButton,
        CreateEmbed, CreateInteractionResponse, CreateInteractionResponseMessage, Error,
    },
    CreateReply,
};
use serde::{Deserialize, Serialize};
use slug::slugify;

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
    let ctfnote_url = &config.ctfnote.ctfnote_url;
    let ctfnote_admin_api_password = &config.ctfnote.ctfnote_admin_api_password;

    let discord_id = ctx.author().id;
    let client = reqwest::Client::new();
    let res = client
        .post(format!("{}/extra/api/admin/link-discord", ctfnote_url))
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
    let ctfnote_url = &config.ctfnote.ctfnote_url;
    let ctfnote_admin_api_password = &config.ctfnote.ctfnote_admin_api_password;

    let discord_id = ctx.author().id;
    let client = reqwest::Client::new();
    let token_endpoint = format!("{}/extra/api/admin/get-token", ctfnote_url);
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
                "<{}/extra/token-login?token={}>\nExpires <t:{}>",
                ctfnote_url, token.token, token.exp
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
            "{}/extra/api/admin/register",
            &config.ctfnote.ctfnote_url
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
    ctf_url: Option<String>,
    logo_url: Option<String>,
    ctftime_url: Option<String>,
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
            "{}/extra/api/admin/role",
            &config.ctfnote.ctfnote_url
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
            "{}/extra/api/admin/upcoming-ctf",
            &config.ctfnote.ctfnote_url
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
        let ctfnote_link = format!(
            "{}/#/ctf/{}-{}",
            config.ctfnote.ctfnote_url,
            ctf.id,
            slugify(&ctf.title)
        );
        let mut embed = CreateEmbed::new()
            .title(&ctf.title)
            .description(&ctf.description)
            .field(
                "Dates",
                format!(
                    "Starts: <t:{}:f>.\n Ends: <t:{}:f>",
                    ctf.start_time.timestamp(),
                    ctf.end_time.timestamp(),
                ),
                true,
            )
            .fields([("Weight", &ctf.weight.to_string(), true)]);
        if ctf.ctftime_url.is_some() {
            embed = embed.url(ctf.ctftime_url.unwrap());
        }
        if ctf.logo_url.is_some() {
            embed = embed.thumbnail(&ctf.logo_url.unwrap());
        }
        if ctf.ctf_url.is_some() {
            embed = embed.field("CTF Page", ctf.ctf_url.unwrap(), true);
        }
        ctx.send(
            CreateReply::default()
                .embed(embed)
                .components(vec![CreateActionRow::Buttons(vec![
                    CreateButton::new(custom_id.clone()).label("Join on CTFNote"),
                    CreateButton::new_link(ctfnote_link).label("View on CTFNote"),
                ])]),
        )
        .await?;

        let mut custom_id_ = custom_id.clone();
        while let Some(mci) = ComponentInteractionCollector::new(ctx)
            .guild_id(ctx.guild_id().unwrap())
            .channel_id(ctx.channel_id())
            .timeout(std::time::Duration::from_secs(30 * 24 * 60 * 60)) // 30 days
            .filter(move |mci| mci.data.custom_id == custom_id_.clone())
            .await
        {
            let res = client
                .post(format!(
                    "{}/extra/api/admin/add-to-ctf",
                    &config.ctfnote.ctfnote_url
                ))
                .basic_auth("admin", Some(&config.ctfnote.ctfnote_admin_api_password))
                .json(&AddDiscordUserToCtfRequest {
                    discord_id: mci.user.id.to_string(),
                    ctf_id: ctf.id,
                })
                .send()
                .await?;
            let response = res.json::<AddDiscordUserToCtfResponse>().await?;
            mci.create_response(
                ctx,
                CreateInteractionResponse::Message(
                    CreateInteractionResponseMessage::default()
                        .allowed_mentions(CreateAllowedMentions::new().users(vec![mci.user.id]))
                        .content(format!("<@{}> {}", mci.user.id, response.message)),
                ),
            )
            .await?;

            custom_id_ = custom_id.clone();
        }
    }

    Ok(())
}

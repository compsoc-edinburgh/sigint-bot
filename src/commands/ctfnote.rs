use poise::{serenity_prelude::Error, CreateReply};
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
            ctx.send(
                CreateReply::default()
                    .ephemeral(true)
                    .content(format!("<{}/token-login?token={}>\nExpires <t:{}>", ctfnote_extra_url, token.token, token.exp)),
            )
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
    #[description = "Username"] username: Option<String>
) -> Result<(), Error> {
    let config = &ctx.data().config;

    // only runs in team channel
    let team_channel_id = config.team_channel_id;
    let channel_id = ctx.channel_id().get();
    if channel_id != team_channel_id {
        ctx.reply(format!("You can only run this command in team channel <#{}>", team_channel_id)).await?;
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
        .post(format!("{}/api/admin/register", &config.ctfnote.ctfnote_extra_url))
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

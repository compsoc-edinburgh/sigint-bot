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
struct CtfnoteGenerateJwtRequest {
    discord_id: String,
}

#[derive(Deserialize)]
struct CtfnoteGenerateJwtResponse {
    jwt: Option<CtfnoteGenerateJwtResponseJwt>,
    message: String,
}

#[derive(Deserialize)]
struct CtfnoteGenerateJwtResponseJwt {
    token: String,
    claim: JwtClaim,
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
    let res = client
        .post(format!("{}/api/admin/generate-jwt", ctfnote_extra_url))
        .basic_auth("admin", Some(ctfnote_admin_api_password))
        .json(&CtfnoteGenerateJwtRequest {
            discord_id: discord_id.to_string(),
        })
        .send()
        .await?;
    let response = res.json::<CtfnoteGenerateJwtResponse>().await?;
    let jwt = response.jwt;
    match jwt {
        Some(jwt) => {
            ctx.send(
                CreateReply::default()
                    .ephemeral(true)
                    .content(format!("<{}/token-login#token={}>\nExpires <t:{}>", ctfnote_extra_url, jwt.token, jwt.claim.exp)),
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

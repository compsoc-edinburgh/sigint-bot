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
    let ctfnote_admin_api_endpoint = &config.ctfnote_admin_api_endpoint;
    let ctfnote_admin_api_password = &config.ctfnote_admin_api_password;

    let discord_id = ctx.author().id;
    let client = reqwest::Client::new();
    let res = client.post(format!("{}/link-discord", ctfnote_admin_api_endpoint))
        .basic_auth("admin", Some(ctfnote_admin_api_password))
        .json(&CtfnoteLinkRequest {
            token: token.to_string(),
            discord_id: discord_id.to_string(),
        })
        .send()
        .await?;
    let message = res.json::<CtfnoteLinkResponse>().await?;

    ctx.send(CreateReply::default().ephemeral(true).content(format!("{}", message.message))).await?;
    Ok(())
}

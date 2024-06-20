use poise::serenity_prelude::Error;
use serde::{Deserialize, Serialize};

use crate::{ConfigContainer, Context};

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
    let data = ctx.discord().data.write().await;
    let config = data
        .get::<ConfigContainer>()
        .expect("Could not get config from context");
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

    ctx.defer_ephemeral().await?;
    ctx.say(format!("{}", message.message)).await?;
    Ok(())
}

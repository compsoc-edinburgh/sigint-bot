use crate::Context;
use poise::{
    command,
    serenity_prelude::{GuildId, RoleId, Error},
};
use tracing::{error, info};


#[command(prefix_command, dm_only)]
pub async fn welcome(
    ctx: Context<'_>,
    #[description = "flag"] flag: String,
) -> std::result::Result<(), Error> {
    let config = &ctx.data().config;
    let sigint_guild = GuildId::new(config.guild_id);
    let welcome_role = RoleId::new(config.welcome.role_id);

    let message = if ctx
        .author()
        .has_role(&ctx.http(), sigint_guild, welcome_role)
        .await?
    {
        "You already have the \"Curious Hacker\" role."
    } else if flag == config.welcome.flag {
        // Add role to the person DM
        match sigint_guild
            .member(&ctx.http(), ctx.author().id)
            .await
        {
            Ok(member) => {
                member.add_role(&ctx.http(), welcome_role).await?;
                info!(
                    "awarded \"Curious Hacker\" role to {}.",
                    ctx.author().name
                );

                "Congratulations! You have earned the \"Curious Hacker\" role!"
            }
            Err(Error::Http(_)) => {
                info!(
                    "non-member {} attempted `welcome` command.",
                    ctx.author().name
                );
                "Please join the SIGINT server first! https://discord.gg/WynY7FD3HP"
            }
            err => {
                error!("welcome command member retrieval failed {:?}!", err);
                "An error has occurred, please contact SIGINT admin"
            }
        }
    } else {
        "I don't think that is the right flag... Try harder!"
    };

    ctx.say(message).await?;
    Ok(())
}

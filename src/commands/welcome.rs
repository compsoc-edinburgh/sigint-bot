use crate::ConfigContainer;
use serenity::framework::standard::{macros::command, Args, CommandResult};
use serenity::model::prelude::*;
use serenity::prelude::*;

#[command]
pub async fn welcome(ctx: &Context, msg: &Message, mut args: Args) -> CommandResult {
    let flag = args.single::<String>()?;

    let data = ctx.data.write().await;
    let config = data
        .get::<ConfigContainer>()
        .expect("Could not get config from context");
    let sigint_guild = GuildId(config.guild_id);
    let welcome_role = RoleId(config.welcome.role_id);

    let message = if msg
        .author
        .has_role(&ctx.http, sigint_guild, welcome_role)
        .await?
    {
        "You already have the \"Curious Hacker\" role."
    } else if flag == config.welcome.flag {
        // Add role to the person DM
        match sigint_guild.member(&ctx.http, msg.author.id).await {
            Ok(mut member) => {
                member.add_role(&ctx.http, welcome_role).await?;
                "Congratulations! You have earned the \"Curious Hacker\" role!"
            }
            Err(SerenityError::Http(_)) => {
                "Please join the SIGINT server first! https://discord.gg/WynY7FD3HP"
            }
            _ => "An error has occurred, please contact SIGINT admin",
        }
    } else {
        "I don't think that is the right flag... Try harder!"
    };

    msg.author
        .direct_message(&ctx, |m| m.content(message))
        .await?;
    Ok(())
}

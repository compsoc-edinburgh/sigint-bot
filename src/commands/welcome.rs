use crate::{WelcomeFlagContainer, SIGINT_GUILD_ID};
use serenity::framework::standard::{macros::command, Args, CommandResult};
use serenity::model::prelude::*;
use serenity::prelude::*;

const WELCOME_ROLE_ID: RoleId = RoleId(885293563380895754);

#[command]
pub async fn welcome(ctx: &Context, msg: &Message, mut args: Args) -> CommandResult {
    let flag = args.single::<String>()?;

    let data = ctx.data.write().await;
    let welcome_flag = data
        .get::<WelcomeFlagContainer>()
        .expect("Could not get welcome flag from context");

    if flag == *welcome_flag {
        // Add role to the person DM
        let mut member = SIGINT_GUILD_ID.member(&ctx.http, msg.author.id).await?;
        member.add_role(&ctx.http, WELCOME_ROLE_ID).await?;

        msg.author
            .direct_message(&ctx, |m| {
                m.content("Congratulations! You have earned the \"Welcome Solver\" role!")
            })
            .await?;
    } else {
        msg.author
            .direct_message(&ctx, |m| {
                m.content("I don't think that is the right flag... Try harder!")
            })
            .await?;
    }

    Ok(())
}

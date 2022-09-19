use crate::Context;

#[poise::command(slash_command, prefix_command)]
pub async fn register_slash_commands(
    ctx: Context<'_>,
) -> Result<(), poise::serenity_prelude::Error> {
    poise::builtins::register_application_commands_buttons(ctx).await?;
    Ok(())
}

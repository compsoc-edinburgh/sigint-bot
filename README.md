# SIGINT Discord Bot

This Discord bot manages SIGINT's Discord presence and roles.

It requires a `config.toml` file with the following environment variables defined:

```
discord_token = <discord token>
client_id = <client id, not actually used>
guild_id = <guild id for your server>
notification_channel_id = <channel id for ctf time notifications>
notification_role_id = <role id that is pinged for each query>
ctftime_loop_seconds = 120 <time in seconds between each poll to ctftime>
team_channel_id = <channel id of trusted team members have access to, command to create CTFNote account can run in this channel>

[ctfnote]
ctfnote_url = "http://localhost:8080"
ctfnote_admin_api_password = "admin_api_password"

[welcome]
role_id = 1021415544919961693 <role id given when flag is solved>
flag = "sigint{test}"

```

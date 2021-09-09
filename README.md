# SIGINT Discord Bot

This Discord bot manages SIGINT's Discord presence and roles.

It requires a `config.toml` file with the following environment variables defined:

```
discord_token = <Developer Token>
guild_id = <Server ID>

[welcome]
flag = "sigint{test}"
role_id = <ID of role to assign>
```

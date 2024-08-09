# 1N4

> Your resident M41D Unit, here to help with your server.

1N4, pronounched 'ee-nah' (/inä/), is a Discord bot made for a private Minecraft SMP.

It provides moderation capabilities and various useful features, while also including built-in lore through the official bot. None of the bot's lore will be provided through this repository for the sake of secrecy. Spoilers are no fun!

1N4 probably isn't for you, as it is highly specialized for what our server needs as a group.

## Usage

You may invite 1N4 using her [invitation link](https://discord.com/oauth2/authorize?client_id=1265640027896021106), or you can install and run her directly through Git and Cargo.

```sh
git clone https://github.com/Jaxydog/1N4.git
cd 1N4
cargo build --release
./target/release/ina --help
```

If you are running your own instance of 1N4, ensure that the following environment variables are set to get full functionality:

- `DISCORD_TOKEN` - The bot's Discord token, which is required to connect to their API.
- `DEVELOPMENT_GUILD_ID` - The bot's 'development guild', which is where the bot will accept interactions from when not running in release mode.
- `DEVELOPMENT_CHANNEL_ID` - The bot's 'development channel', which is where things like error logs are sent. 1N4 assumes that the given channel is within the guild specified by `DEVELOPMENT_GUILD_ID`

## Contributing

Contributions are always welcome! If you're interested in helping development, please read 1N4's [contribution guidelines](./doc/CONTRIBUTING.md).

If you ever need help, feel free to contact me here or on Discord, and I will do my best to assist you <3

## License

1N4 is free software: you can redistribute it and/or modify it under the terms of the GNU Affero General Public License as published by the Free Software Foundation, either version 3 of the License, or (at your option) any later version.

1N4 is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the GNU Affero General Public License for more details.

You should have received a copy of the GNU Affero General Public License along with 1N4 (located within [LICENSE](./LICENSE)). If not, see <https://www.gnu.org/licenses/>.

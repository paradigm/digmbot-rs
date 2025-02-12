# digmbot-rs

## What is it

Discord bot written in Rust with a focus on LLM experimentation.

## Setup and configuration

As this code base is intended more as a experimentation platform rather than a production service, there's no commitment to the following setup and configuration details being valid for any period going forward.

### Compilation and Installation

Clone the source code:

```
git clone https://github.com/paradigm/digmbot-rs
cd digmbot-rs
```

Install the following build dependencies:

- A Rust build toolchain, including `cargo`.
- A linker for the Rust toolchain, such as provided by `gcc`.
- `pkg-config`

And the following build _and_ runtime dependencies:

- OpenSSL libraries, such as provided by `libssl-dev`, for TLS to Discord.

To build without full optimizations (which builds faster and expedites development):

```
cargo build
```

To build with full optimizations (for production release):

```
cargo build --release
```

To (build and) run a non-release build:

```
cargo run
```

To (build and) run the production build:

```
cargo run --release
```

To install digmbot somewhere, copy the release build from `./target/release/digmbot` to the target location.  From there you can just execute the binary.

### Configuration

Create a configuration file at `~/.config/digmbot/config.toml` with the following, updating accordingly:

```
[general]
# Discord token
# See e.g. https://www.writebots.com/discord-bot-token/
discord_token = "<TODO>"
# List of Discord accounts with full permissions to manipulate the bot.
# Use profile names, e.g. as shown in DMs, rather than per-guild names.
bot_owners = []
# A string (usually just a character) to prefix before commands
# e.g. `"!"` would result in commands like `!help`
command_prefix = "!"
# Do not send notifications to users more than once in this time period to avoid spamming
notification_limit_seconds = 900

[history]
# When a given digmbot session first sees a room, how many messages to backfill into its history.
# Requesting too many may result in Discord throttling the bot
channel_backfill_message_count = 50
# Maximum number of messages to store per room.
# More results in more memory usage
channel_max_message_count = 100

[llm_general]
# URL of OpenAI-compatible LLM chat API
chat_url = "http://127.0.0.1:11434/api/generate"
# URL of OpenAI-compatible LLM completion API
completion_url = "http://127.0.0.1:11434/api/chat"

[llm_reply]
# When the bot receives an `@<username>` or reply, it replies with an
# LLM-generated message with these settings.
model_name = "<TODO>"
context_size = 8192
temperature = 0.8
# The following substitutions are dynamically performed:
# - `{bot}` is replaced with the bot name
# - `{user}` is replaced with the user whose message is being replied to
system = "You are {bot}, a Discord bot.  You are helpful, friendly, and kind.  Your source code is hosted at https://github.com/paradigm/digmbot-rs"

[llm_permission_denied]
# When a user with insufficient bot permissions (e.g. not in `bot_owners`)
# tries to do something they're not allowed to do, an LLM-generated reply is
# generated with these settings.
model_name = "chat"
context_size = 8192
temperature = 0.8
# - `{bot}` is replaced with the bot name
# - `{user}` is replaced with the user whose message is being replied to
system = "You are {bot}, a Discord bot.  {user} just requested an operation to which they do not have permissions.  Patiently explain to them that you're unable to proceed with their request."
```

### Architecture

```
$ tree src
src
├── config.rs -- configuration data
├── context.rs -- data shared across events
├── event.rs -- discord event
├── handler.rs -- discord even thandler
├── helper.rs -- miscellaneous helper code
├── llm.rs -- LLM code
├── logging.rs -- logging
├── main.rs -- main entry point
├── persistent_state.rs -- data which persists across sessions
├── plugin -- plugins
│   ├── mod.rs -- plugin system entry point
│   ├── *.rs -- plugins
└── volatile_state.rs -- data which does not persist across sessions
```

### Key developer concepts

- This bot uses the [Serenity crate](https://crates.io/crates/serenity), which is built around a callback architecture for various Discord events.  However, this does not play cleanly with our plugin system.  Thus, within `event.rs` we convert it to an `Event` enum which is passed to the plugins.
- Serenity requires we be async.
    - Under-the-hood, this results in the code being broken up into components which are individually scheduled.
    - Any function which performs I/O (such as API calls to Discord or filesystem system calls) must be prefixed with the `async` key word
    - Calls to such functions must be followed by `.await`
    - Traits and impls with async functions must use the `#[serenity::async_trait]` macro.
- Data shared across events is stored with a common `ctx: &Context`
    - `cfg` contains configuration data, stored in `config.toml`
    - `pstate` contains data which persists across sessions, stored in `state.toml`
    - `vstate` contains data which does  not persists across sessions
    - `cache` is Serenity-cached data.  Pass to Serenity functions.
    - `http` is Serenity subsystem for API requests to Discord.  Pass to Serenity functions.
    - `cache_http` is Serenity subsystem to check the cache then, if it's missing, reach out to Discord.  Pass to Serenity functions.
- Most of the bot's features are implemented via a plugin system
    - `plugin/mod.rs` provides a `Plugin` trait that must be implemented for all plugins.  See its comments.
    - `plugin/mod.rs` has a `plugins()` function which lists enabled plugins.  Add any new plugin to it, or comment/remove any which you'd like to disable.

### Feature submission ideas

- CI
    - Github actions to perform `rustfmt`, `cargo clippy`, etc checks
- Configuration
    - Explicitly support non-Linux platforms, making code changes if necessary, and updating this `README.md` accordingly
- Discord
    - LLM plugin generated responses may be over Discord message length (2k characters); if so, split into multiple messages.
- LLM tech
    - Dynamically calculate exact number of room history messages to put into the model's context based on context size configuration
    - Implement LLM function calling such that the bot can do things like list available channels, users, and create messages.  Consider, for example, a reminder system.
    - Implement [vision model and image support](https://ollama.com/blog/vision-models)
    - Vector database or other system so the bot can maintain its own state
    - Model Context Protocol support
- New commands
    - Add a `;rebuild` to have the bot to pull down the latest commit and, if it differs from the running version, rebuild and re-exec itself.
    - Add a `;puppet` to have the bot send a given message to a given channel.  Intended for use in DMs.  Constrain to configured whitelist accounts.
    - Add a `;act` to prompt the bot to do some LLM-driven action once things like function calling are in place such that it can decide things like the Discord room
    - Add a `;join-vc` to have the bot to join a VC channel, continuously convert the audio to text (e.g. with Whisper), and feed text following keywords into an LLM.  Consider hands-free Alexa/HeyGoogle/etc style commands.

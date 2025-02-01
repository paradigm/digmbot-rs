# digmbot-rs

## What is it

Discord bot written in Rust with a focus on LLM experimentation.

## Setup and configuration

As this code base is intended more as a experimentation platform rather than a production service, there's no commitment to the following setup and configuration details being valid for any period going forward.

### Compilation and Installation

```
git clone https://github.com/paradigm/digmbot-rs
cd digmbot-rs
cargo build --release
cp ./target/release/digmbot /path/to/install/location
```

### Discord token

To have digmbot connect to a Discord server, you'll need a discord bot token.

- Login to discord.com and go to [the developer application page](https://discord.com/developers/applications/).
- Click "New Application" in the corner and follow the prompts to create a Discord application.
- Within the application, look for "Bot" and follow the prompts to create a bot for the application.
- Under the application's bots section, click on "Click to Reveal Token" to acquire the token string.
- Place token string within a file at `~/.config/digmbot/discord_token` (on Linux) or the equivalent (on other platforms)
- Optionally, add the bot to a Discord server.  Alternatively, just DM with it.

### LLM API settings

To have the bot respond to `@`-mentions and replies with an LLM-generated response, it needs to be configured to use some LLM completion API end point.

You can use services provided by LLM vendors such as OpenAI and Anthropic, or self-host with something like [ollama](https://ollama.com/).

Create a basic json file at `~/.config/digmbot/llm_settings.json` (on Linux) or the equivalent (on other platforms) with the following json keys:

- `completion_url`: The URL of a OpenAI-derived standard LLM completion point such as an ollama instance.
- `model_name`: The name of the model you'd like to run on the completion endpoint.
- `system`: The system prompt.  This instructs the LLM on how to behave.  Optionally, you may include a `{}` within the prompt and the bot will replace it with the bot's name as provided by Discord.
- `context_size`: How many tokens worth of context to the LLM will use.  This determines things like the amount of channel history the bot "backreads" before generating its response.
- Text which provides structure to the LLM's prompt.  For most major public models, you'll find these in the LLM model's documentation.
    - `system_msg_start`: Text to place before the system prompt.
    - `system_msg_end`: Text to place after the system prompt.
    - `user_msg_start`: Text to place before a message from the users in the Discord channel
    - `user_msg_end`: Text to place after a message from the users in the Discord channel
    - `bot_msg_start`: Text to place before a message from the bot in the Discord channel
    - `bot_msg_end`: Text to place after a message from the bot in the Discord channel

For example:

```
{
	"completion_url": "http://example.com:11434/api/generate",
	"model_name": "example",
	"system": "You are {}, a Discord bot.  You creatively segue any discussion topic to your enjoyment of working with the Rust programming language.",
	"context_size": 8192,
	"system_msg_start": "<|start_header_id|>system\n",
	"system_msg_end": "<|end_header_id|>",
	"user_msg_start": "<|start_header_id|>user\n",
	"user_msg_end": "<|end_header_id|>",
	"bot_msg_start": "<|start_header_id|>assistant\n",
	"bot_msg_end": "<|end_header_id|>"
}
```

### Architecture

```
$ tree src
src
├── event.rs
├── helper.rs
├── main.rs
└── plugin
    ├── mod.rs
    └── *.rs
```

- `main.rs`: The main bot entry point.
- `event.rs`: Discord events.
- `helper.rs`: Miscellaneous bits of auxiliary code
- `plugin/mod.rs`: Plugin system entry point.
- `plugin/*.rs`: Individual plugins


### Key developer concepts

- This bot uses the [Serenity crate](https://crates.io/crates/serenity), which is built around a callback architecture for various Discord events.  However, this does not play cleanly with our plugin system.  Thus, within `event.rs` we convert it to an `Event` enum which is passed to the plugins.
- Serenity requires we be async.
    - Under-the-hood, this results in the code being broken up into components which are individually scheduled.
    - Any function which performs I/O (such as API calls to Discord or filesystem system calls) must be prefixed with the `async` key word
    - Calls to such functions must be followed by `.await`
    - Traits and impls with async functions must use the `#[serenity::async_trait]` macro.
- Serenity provides a `ctx: Context` in each callback which gets embedded within each `Event` and forwarded to each plugin.
    - `ctx.data` provides state information which can persist across events.
    - Due to the combination of Serenity's async requirement and Rust's type enforcement around thread safety, accessing the state information requires a bit of an incantation.
    - While you're welcome to decipher it, also feel free to just copy examples in provided plugins.
- Most of the bot's features are implemented via a plugin system
    - `plugin/mod.rs` provides a `Plugin` trait that must be implemented for all plugins.  See its comments.
    - `plugin/mod.rs` has a `plugins()` function which lists enabled plugins.  Add any new plugin to it, or comment/remove any which you'd like to disable.
- Configuration should be stored in `~/.config/digmbot/` on Linux or the equivalent on other platforms.
- Convention is for special commands to be prefixed with a semicolon.

### Feature submission ideas

- CI
    - Github actions to perform `rustfmt`, `cargo clippy`, etc checks
- Configuration
    - Bot owner whitelist configuration
    - Explicitly support non-Linux platforms, making code changes if necessary, and updating this `README.md` accordingly
- LLM tech
    - Rather than requiring users configure LLM special tokens, the LLM completion API call should use the `messages` feature to provide the message history with corresponding roles.  [See ollama's documentation](https://ollama.com/blog/openai-compatibility).  This should both simplify the user-facing configuration and code base.
    - Refactor out the LLM completion logic into its own module that the LLM plugin calls such that we can use LLMs for other plugins/features.
    - Dynamically calculate exact number of room history messages to put into the model's context based on context size configuration
    - Implement LLM function calling such that the bot can do things like list available channels, users, and create messages.  Consider, for example, a reminder system.
    - Implement [vision model and image support](https://ollama.com/blog/vision-models)
    - Vector database or other system so the bot can maintain its own state
    - Model Context Protocol support
- New commands
    - Add a `;reload` to have the bot reload itself, such as if on-disk configuration changed
    - Add a `;rebuild` to have the bot to pull down the latest commit and, if it differs from the running version, rebuild and re-exec itself.
    - Add a `;puppet` to have the bot send a given message to a given channel.  Intended for use in DMs.  Constrain to configured whitelist accounts.
    - Add a `;act` to prompt the bot to do some LLM-driven action once things like function calling are in place such that it can decide things like the Discord room
    - Add a `;join-vc` to have the bot to join a VC channel, continuously convert the audio to text (e.g. with Whisper), and feed text following keywords into an LLM.  Consider hands-free Alexa/HeyGoogle/etc style commands.
    - Add a `;llm-model` command to change the LLM model on-the-fly.  Constrain to configured whitelist accounts.
    - Add a `;llm-ctx` command to change the LLM context size on-the-fly.  Constrain to configured whitelist accounts.

# Top Agent - Local AI Coding Agent

A minimal coding agent that runs locally in your workspace. It uses an LLM via OpenRouter to execute file operations, shell commands, and git actions.

## Current Status

Early-phase project. Works for basic file operations, shell commands, git workflows, and task planning. Not production-tested. Use with caution.

## Prerequisites

- Rust (1.75+)
- OpenRouter API key

## Install Rust

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source ~/.cargo/env
```

## Quick Start

```bash
# Clone and install
git clone https://github.com/TopWayAI/topagent.git
cd topagent
cargo install --path crates/topagent-cli

# Set your API key (do this once per shell session)
export OPENROUTER_API_KEY=your_key_here

# Run from your project directory
cd /path/to/your/project
topagent "summarize this project"
```

The agent uses MiniMax M2.7 by default.

## Example Commands

```bash
# Inspect a project
topagent "give me a summary"

# Check git status
topagent "show git status"

# Make a small edit
topagent "add a TODO comment to src/main.rs"
```

## Alternative: Run Without Installing

If you don't want to install, run the binary directly:

```bash
cargo build --release
cd /path/to/your/project
/path/to/topagent/target/release/topagent --api-key YOUR_KEY "summarize this project"
```

## Optional: Workspace Files

**TOPAGENT.md** - Project instructions (optional, place in workspace root):

```markdown
# Project Instructions

- Use TypeScript, not JavaScript
- Run tests before committing
```

## Telegram Bot (Optional)

Run the agent as a Telegram bot with long polling.

### Setup

1. Create a bot: open Telegram, search for **BotFather**, send `/newbot`
2. Copy the bot token (e.g. `123456:ABCdef...`)
3. Export: `export TELEGRAM_BOT_TOKEN=your_token`
4. Make sure no webhook is active (BotFather will tell you if one is set)
5. Run:
   ```bash
   topagent telegram serve --cwd /path/to/your/project
   ```
6. Open Telegram, find your bot, send a private text message

### First-version limitations
- **private chats only** (groups/supergroups ignored)
- **text messages only** (photos/docs/other types get a clear reply)
- **in-memory sessions** (history clears on restart)

### Built-in commands
- `/start` - show bot info
- `/help` - show usage
- `/reset` - clear conversation history for this chat

### CLI Options

```bash
topagent --help

Options:
  --api-key KEY      OpenRouter API key (or set OPENROUTER_API_KEY env var)
  --model MODEL      Model to use (default: minimax/minimax-m2.7)
  --cwd DIR          Working directory
  --max-steps N      Max steps (default: 50)
  --max-retries N    Retries (default: 3)
  --timeout-secs N   Timeout (default: 120)
```

## Safety

This agent can read/write/edit files, run shell commands, and execute git operations. Only run in directories you trust.

## Uninstall

```bash
cargo uninstall topagent-cli
```

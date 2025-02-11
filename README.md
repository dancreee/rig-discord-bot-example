# Introduction

Welcome to the Rust Discord Bot documentation. This bot leverages the Rig library to provide AI-powered assistance.

# Installation

To install the bot, clone the repository and run `cargo run`.

# Usage

Use the `/hello` command to greet the bot and `/ask` to ask Rust-related questions.

# Advanced Features

The bot supports Retrieval-Augmented Generation (RAG) to answer questions based on this documentation.

# Environment Variables

The following environment variables need to be set in your `.env` file:

1. `DISCORD_TOKEN`: Your Discord bot token
   - Required for bot authentication with Discord
   - Format: Starts with "MT" followed by numbers and letters

2. `OPENAI_API_KEY`: Your OpenAI API key
   - Required for AI functionality and embeddings
   - Format: Starts with "sk-" followed by a long string of characters

# Tutorial

[Medium Tutorial](https://medium.com/@0thTachi/build-an-ai-discord-bot-in-rust-with-rig-a-step-by-step-guide-7410107ff590) by Tachi here including some details on how to setup the discord bot and the .env file
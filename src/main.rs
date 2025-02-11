// main.rs

mod rig_agent;
mod chat_history;

use anyhow::Result;
use serenity::async_trait;
use serenity::model::application::command::Command;
use serenity::model::application::interaction::Interaction;
use serenity::model::gateway::Ready;
use serenity::model::channel::Message;
use serenity::prelude::*;
use serenity::model::application::command::CommandOptionType;
use std::env;
use std::sync::Arc;
use tracing::{error, info, debug};
use rig_agent::RigAgent;
use dotenv::dotenv;

// Define a key for storing the bot's user ID in the TypeMap
struct BotUserId;

impl TypeMapKey for BotUserId {
    type Value = serenity::model::id::UserId;
}

struct Handler {
    rig_agent: Arc<RigAgent>,
}

#[async_trait]
impl EventHandler for Handler {
    async fn interaction_create(&self, ctx: Context, interaction: Interaction) {
        debug!("Received an interaction");
        if let Interaction::ApplicationCommand(command) = interaction {
            debug!("Received command: {}", command.data.name);

            // Immediately acknowledge the interaction
            if let Err(e) = command.defer(&ctx.http).await {
                error!("Failed to defer response: {:?}", e);
                return;
            }

            let content = match command.data.name.as_str() {
                "hello" => "Hello! I'm your helpful Rust and Rig-powered assistant. How can I assist you today?".to_string(),
                "ask" => {
                    let query = command
                        .data
                        .options
                        .get(0)
                        .and_then(|opt| opt.value.as_ref())
                        .and_then(|v| v.as_str())
                        .unwrap_or("What would you like to ask?");
                    debug!("Query: {}", query);
                    match self.rig_agent.process_message(&command.user.id.to_string(), query).await {
                        Ok(response) => response,
                        Err(e) => {
                            error!("Error processing request: {:?}", e);
                            format!("Error processing request: {:?}", e)
                        }
                    }
                }
                _ => "Not implemented :(".to_string(),
            };

            debug!("Sending response: {}", content);

        // Split message if it's too long (Discord limit is 2000 characters)
        if content.len() > 2000 {
            // Send first part as edit to original response
            if let Err(why) = command.edit_original_interaction_response(&ctx.http, |response| {
                response.content(content[..1997].to_string() + "...")
            }).await {
                error!("Cannot send first part of response: {:?}", why);
                return;
            }

            // Send remaining content as follow-up messages
            let remaining = content[1997..].to_string();
            for chunk in remaining.chars().collect::<Vec<char>>().chunks(2000) {
                let chunk_content: String = chunk.iter().collect();
                if let Err(why) = command.create_followup_message(&ctx.http, |message| {
                    message.content(chunk_content)
                }).await {
                    error!("Cannot send follow-up chunk: {:?}", why);
                }
            }
        } else {
            // Send as normal if content is within limits
            if let Err(why) = command.edit_original_interaction_response(&ctx.http, |response| {
                response.content(content)
            }).await {
                error!("Cannot send follow-up response: {:?}", why);
            } else {
                debug!("Response sent successfully");
                }
            }
        }
    }

    async fn message(&self, ctx: Context, msg: Message) {
        if msg.mentions_me(&ctx.http).await.unwrap_or(false) {
            debug!("Bot mentioned in message: {}", msg.content);

            let bot_id = {
                let data = ctx.data.read().await;
                data.get::<BotUserId>().copied()
            };

            if let Some(bot_id) = bot_id {
                let mention = format!("<@{}>", bot_id);
                let content = msg.content.replace(&mention, "").trim().to_string();

                debug!("Processed content after removing mention: {}", content);

                match self.rig_agent.process_message(&msg.author.id.to_string(), &content).await {
                    Ok(response) => {
                        if let Err(why) = msg.channel_id.say(&ctx.http, response).await {
                            error!("Error sending message: {:?}", why);
                        }
                    }
                    Err(e) => {
                        error!("Error processing message: {:?}", e);
                        if let Err(why) = msg
                            .channel_id
                            .say(&ctx.http, format!("Error processing message: {:?}", e))
                            .await
                        {
                            error!("Error sending error message: {:?}", why);
                        }
                    }
                }
            } else {
                error!("Bot user ID not found in TypeMap");
            }
        }
    }

    async fn ready(&self, ctx: Context, ready: Ready) {
        info!("{} is connected!", ready.user.name);

        {
            let mut data = ctx.data.write().await;
            data.insert::<BotUserId>(ready.user.id);
        }

        let commands = Command::set_global_application_commands(&ctx.http, |commands| {
            commands
                .create_application_command(|command| {
                    command
                        .name("hello")
                        .description("Say hello to the bot")
                })
                .create_application_command(|command| {
                    command
                        .name("ask")
                        .description("Ask the bot a question")
                        .create_option(|option| {
                            option
                                .name("query")
                                .description("Your question for the bot")
                                .kind(CommandOptionType::String)
                                .required(true)
                        })
                })
        })
        .await;

        println!("Created the following global commands: {:#?}", commands);
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    dotenv().ok();

    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::DEBUG)
        .init();

    let token = env::var("DISCORD_TOKEN").expect("Expected DISCORD_TOKEN in environment");

    let rig_agent = Arc::new(RigAgent::new().await?);

    let intents = GatewayIntents::GUILD_MESSAGES
        | GatewayIntents::DIRECT_MESSAGES
        | GatewayIntents::MESSAGE_CONTENT;

    let mut client = Client::builder(&token, intents)
        .event_handler(Handler {
            rig_agent: Arc::clone(&rig_agent),
        })
        .await
        .expect("Err creating client");

    if let Err(why) = client.start().await {
        error!("Client error: {:?}", why);
    }

    Ok(())
}
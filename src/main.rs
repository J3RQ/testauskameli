use regex::Regex;
use std::{env, sync::Arc};

use serenity::async_trait;
use serenity::client::bridge::gateway::ShardManager;
use serenity::model::channel::Message;
use serenity::model::event::ResumedEvent;
use serenity::model::gateway::Ready;
use serenity::prelude::*;
use tracing::{error, info};

mod commands;

pub struct ShardManagerContainer;

impl TypeMapKey for ShardManagerContainer {
    type Value = Arc<Mutex<ShardManager>>;
}

struct Handler;

#[async_trait]
impl EventHandler for Handler {
    async fn ready(&self, _: Context, ready: Ready) {
        info!("Connected as {}", ready.user.name);
    }

    async fn resume(&self, _: Context, _: ResumedEvent) {
        info!("Resumed");
    }

    async fn message(&self, ctx: Context, msg: Message) {
        if msg.author.bot || !msg.mentions_me(&ctx.http).await.unwrap_or(false) {
            return;
        }
        let filetype = if msg.content.contains("```hs") {
            "```hs"
        } else if msg.content.contains("```haskell") {
            "```haskell"
        } else {
            ""
        };
        if filetype.is_empty() {
            // NO MEME?
            let re = Regex::new(r"(?i)no\s+(.*)?\?").unwrap();
            if let Some(cap) = re.captures_iter(&msg.content).next() {
                commands::meme_generator::no(cap.get(1).unwrap().as_str());
                msg.channel_id
                    .send_message(&ctx.http, |m| m.add_file("test.png"))
                    .await
                    .unwrap();
            };
            return;
        } else {
            let code = msg.content[msg.content.find(filetype).unwrap() + filetype.len()
                ..msg.content.rfind("```").unwrap()]
                .to_string();
            info!("Compiling program: {}", &code);
            commands::haskell::compile_and_run(&ctx, msg, &code).await;
        }
    }
}

#[tokio::main]
async fn main() {
    dotenv::dotenv().expect("Failed to load .env file");
    tracing_subscriber::fmt::init();

    let token = env::var("DISCORD_TOKEN").expect("Expected a token in the environment");

    let mut client = Client::builder(&token)
        .event_handler(Handler)
        .await
        .expect("Err creating client");

    {
        let mut data = client.data.write().await;
        data.insert::<ShardManagerContainer>(client.shard_manager.clone());
    }

    let shard_manager = client.shard_manager.clone();

    tokio::spawn(async move {
        tokio::signal::ctrl_c()
            .await
            .expect("Could not register ctrl+c handler");
        shard_manager.lock().await.shutdown_all().await;
    });

    if let Err(why) = client.start().await {
        error!("Client error: {:?}", why);
    }
}

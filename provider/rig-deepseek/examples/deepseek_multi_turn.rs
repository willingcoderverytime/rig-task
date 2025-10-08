use anyhow::Result;
use rig::completion::Chat;
use rig::prelude::*;
use rig::{completion::Prompt, message::*};
use std::io::{self, Write};

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    tracing_subscriber::fmt().init();

    // Create DeepSeek client and agent
    let deepseek_client = rig_deepseek::client::Client::from_env();
    let agent = deepseek_client
        .agent(rig_deepseek::completion::DEEPSEEK_CHAT)
        .preamble("You are a helpful assistant.")
        .build();

    println!("DeepSeek Multi-turn Conversation Example");
    println!("=====================================");
    println!("Enter your messages (type 'quit' to exit):");
    println!();

    let mut conversation_history = vec![];

    loop {
        print!("User: ");
        io::stdout().flush()?;

        let mut user_input = String::new();
        io::stdin().read_line(&mut user_input)?;

        let user_input = user_input.trim();

        if user_input.eq_ignore_ascii_case("quit") {
            println!("Goodbye!");
            break;
        }

        if user_input.is_empty() {
            continue;
        }

        // Add user message to history
        conversation_history.push(Message::user(user_input));

        // Get response from agent
        let response = agent
            .chat(user_input, conversation_history.clone())
            .await?;

        println!("Assistant: {}", response);
        println!();

        // Add assistant response to history
        conversation_history.push(Message::assistant(&response));
    }

    Ok(())
}
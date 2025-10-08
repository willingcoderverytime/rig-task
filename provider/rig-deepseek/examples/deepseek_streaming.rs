use anyhow::Result;
use rig::agent::stream_to_stdout;
use rig::prelude::*;
use rig::streaming::StreamingPrompt;

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    tracing_subscriber::fmt().init();

    // Create DeepSeek client and agent
    let deepseek_client = rig_deepseek::client::Client::from_env();
    let agent = deepseek_client
        .agent(rig_deepseek::completion::DEEPSEEK_CHAT)
        .preamble("You are a helpful assistant. Provide detailed and informative responses.")
        .temperature(0.7)
        .build();

    println!("DeepSeek Streaming Example");
    println!("========================");
    println!("Streaming response for the question: 'Explain the theory of relativity in simple terms'...\n");

    // Stream the response
    let mut stream = agent
        .stream_prompt("Explain the theory of relativity in simple terms")
        .await;

    let res = stream_to_stdout(&mut stream).await?;

    println!("\nToken usage: {usage:?}", usage = res.usage());
    println!("Final response: {message:?}", message = res.response());

    Ok(())
}
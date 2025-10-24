use anyhow::Result;
use rig::agent::stream_to_stdout;
use rig::prelude::*;
use rig::streaming::StreamingPrompt;



#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    tracing_subscriber::fmt()
        .with_file(true)
        .with_thread_ids(true)
        .with_thread_ids(true)
        .init();

    // Create agent with a single context prompt and two tools
    let calculator_agent = rig_ollama::client::Client::new()
        .agent("qwen3:4b")
        .preamble(
            "You are a calculator here to help the user perform arithmetic
            operations. Use the tools provided to answer the user's question.
            make your answer long, so we can test the streaming functionality,
            like 20 words",
        )
        .max_tokens(1024)
        .build();

    println!("Calculate 2 - 5");

    let mut stream = calculator_agent.stream_prompt("Calculate 2 - 5").await;
    // println!("{stream}");
    let res = stream_to_stdout(&mut stream).await?;

    println!("Token usage response: {usage:?}", usage = res.usage());
    println!("Final text response: {message:?}", message = res.response());
    Ok(())
}

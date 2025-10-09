use anyhow::Result;
use rig::completion::Prompt;
use rig::prelude::*;
use rig::{completion::ToolDefinition, tool::Tool};
use serde::{Deserialize, Serialize};
use serde_json::json;

#[derive(Debug, thiserror::Error)]
#[error("Math error")]
struct MathError;

#[derive(Deserialize, Serialize)]
struct Adder;

impl Tool for Adder {
    const NAME: &'static str = "add";
    type Error = MathError;
    type Args = AddOperation;
    type Output = i32;

    async fn definition(&self, _prompt: String) -> ToolDefinition {
        ToolDefinition {
            name: "add".to_string(),
            description: "Add two numbers together".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "x": {
                        "type": "number",
                        "description": "The first number to add"
                    },
                    "y": {
                        "type": "number",
                        "description": "The second number to add"
                    }
                },
                "required": ["x", "y"],
            }),
        }
    }

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        let result = args.x + args.y;
        println!("Adding {} + {} = {}", args.x, args.y, result);
        Ok(result)
    }
}

#[derive(Deserialize, Serialize)]
struct Subtract;

impl Tool for Subtract {
    const NAME: &'static str = "subtract";
    type Error = MathError;
    type Args = SubOperation;
    type Output = i32;

    async fn definition(&self, _prompt: String) -> ToolDefinition {
        ToolDefinition {
            name: "subtract".to_string(),
            description: "Subtract second number from the first number".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "x": {
                        "type": "number",
                        "description": "The number to subtract from"
                    },
                    "y": {
                        "type": "number",
                        "description": "The number to subtract"
                    }
                },
                "required": ["x", "y"],
            }),
        }
    }

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        let result = args.x - args.y;
        println!("Subtracting {} - {} = {}", args.x, args.y, result);
        Ok(result)
    }
}

#[derive(Deserialize, Serialize)]
struct AddOperation {
    x: i32,
    y: i32,
}

#[derive(Deserialize, Serialize)]
struct SubOperation {
    x: i32,
    y: i32,
}

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    tracing_subscriber::fmt().init();

    // Create DeepSeek client and agent with tools
    let deepseek_client = rig_deepseek::client::Client::new("");
    let agent = deepseek_client
        .agent(rig_deepseek::completion::DEEPSEEK_CHAT)
        .preamble("You are a calculator assistant. Use the provided tools to perform arithmetic operations. Explain your steps.")
        .max_tokens(1024)
        .tool(Adder)
        .tool(Subtract)
        .build();

    println!("DeepSeek Tool Usage Example");
    println!("=========================");
    
    let questions = vec![
        "What is 15 + 27?",
        "Calculate 100 - 38",
        "How much is 42 plus 18?",
        "What do you get when you subtract 15 from 50?"
    ];

    for (i, question) in questions.iter().enumerate() {
        println!("\nQuestion {}: {}", i + 1, question);
        match agent.prompt(*question).await {
            Ok(response) => {
                println!("Answer: {}", response);
            }
            Err(e) => {
                println!("Error: {}", e);
            }
        }
    }

    Ok(())
}
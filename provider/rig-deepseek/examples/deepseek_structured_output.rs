use anyhow::Result;
use rig::prelude::*;
use rig::extractor::Extractor;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize, JsonSchema, PartialEq)]
struct Person {
    name: String,
    age: u8,
    occupation: String,
    hobbies: Vec<String>,
}

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    tracing_subscriber::fmt().init();

    // Create DeepSeek client
    let deepseek_client = rig_deepseek::client::Client::new("");
    
    // Create extractor
    let extractor = deepseek_client
        .extractor::<Person>(rig_deepseek::completion::DEEPSEEK_CHAT)
        .build();

    // Test extraction with different prompts
    let test_cases = vec![
        "John Doe is a 30-year-old software engineer who enjoys reading, hiking, and playing guitar.",
        "Mary Smith is 25 years old and works as a graphic designer. Her hobbies include painting, photography, and traveling.",
        "Bob Johnson is a 45-year-old chef who loves cooking, gardening, and fishing."
    ];

    println!("DeepSeek Structured Output Extraction Example");
    println!("===========================================");
    
    for (i, text) in test_cases.iter().enumerate() {
        println!("\nTest case {}: {}", i + 1, text);
        
        match extractor.extract(*text).await {
            Ok(person) => {
                println!("Extracted person:");
                println!("  Name: {}", person.name);
                println!("  Age: {}", person.age);
                println!("  Occupation: {}", person.occupation);
                println!("  Hobbies: {:?}", person.hobbies);
            }
            Err(e) => {
                println!("Extraction failed: {}", e);
            }
        }
    }

    Ok(())
}
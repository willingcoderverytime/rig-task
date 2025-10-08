use futures::StreamExt;
use rig::OneOrMany;
use rig::client::ProviderClient;
use rig::client::ProviderValue;
use rig::completion::{Completion, CompletionRequest, ToolDefinition};
use rig::message::AssistantContent;
use rig::message::Message;
use rig::streaming::StreamingCompletion;
use rig::tool::Tool;
use rig_deepseek::completion::DEEPSEEK_CHAT;
use rig_ollama::MODLE_SUPPORT;
use rig_ollama::embedding::NOMIC_EMBED_TEXT;
use serde::{Deserialize, Serialize};
use serde_json::json;

struct ClientConfig {
    name: &'static str,
    namef: Box<dyn Fn() -> &'static str>,
    factory_env: Box<dyn Fn() -> Box<dyn ProviderClient>>,
    // Not sure where we're going to be using this but I've added it for completeness
    #[allow(dead_code)]
    factory_val: Box<dyn Fn(ProviderValue) -> Box<dyn ProviderClient>>,
    env_variable: &'static str,
    completion_model: Option<&'static str>,
    embeddings_model: Option<&'static str>,
}

impl Default for ClientConfig {
    fn default() -> Self {
        Self {
            name: "",
            namef: Box::new(|| panic!("Not implemented")),
            factory_env: Box::new(|| panic!("Not implemented")),
            factory_val: Box::new(|_| panic!("Not implemented")),
            env_variable: "",
            completion_model: None,
            embeddings_model: None,
        }
    }
}

impl ClientConfig {
    fn is_env_var_set(&self) -> bool {
        self.env_variable.is_empty() || std::env::var(self.env_variable).is_ok()
    }

    fn factory_env(&self) -> Box<dyn ProviderClient + '_> {
        self.factory_env.as_ref()()
    }
}

fn providers() -> Vec<ClientConfig> {
    vec![
        ClientConfig {
            name: "Deepseek",
            factory_env: Box::new(rig_deepseek::client::Client::from_env_boxed),
            factory_val: Box::new(rig_deepseek::client::Client::from_val_boxed),
            env_variable: "DEEPSEEK_API_KEY",
            completion_model: Some(DEEPSEEK_CHAT),
            ..Default::default()
        },
        ClientConfig {
            name: "Ollama",
            factory_env: Box::new(rig_ollama::client::Client::from_env_boxed),
            factory_val: Box::new(rig_ollama::client::Client::from_val_boxed),
            env_variable: "OLLAMA_ENABLED",
            completion_model: Some(MODLE_SUPPORT),
            embeddings_model: Some(NOMIC_EMBED_TEXT),
            ..Default::default()
        },
    ]
}

async fn test_completions_client(config: &ClientConfig) {
    let client = config.factory_env();

    let Some(client) = client.as_completion() else {
        return;
    };

    let model = config
        .completion_model
        .unwrap_or_else(|| panic!("{} does not have completion_model set", config.name));

    let model = client.completion_model(model);

    let resp = model
        .completion_request(Message::user("Whats the capital of France?"))
        .send()
        .await;

    assert!(
        resp.is_ok(),
        "[{}]: Error occurred when prompting, {}",
        config.name,
        resp.err().unwrap()
    );

    let resp = resp.unwrap();

    match resp.choice.first() {
        AssistantContent::Text(text) => {
            assert!(text.text.to_lowercase().contains("paris"));
        }
        _ => {
            unreachable!(
                "[{}]: First choice wasn't a Text message, {:?}",
                config.name,
                resp.choice.first()
            );
        }
    }
}

#[tokio::test]
#[ignore]
async fn test_completions() {
    for p in providers().into_iter().filter(ClientConfig::is_env_var_set) {
        test_completions_client(&p).await;
    }
}

async fn test_tools_client(config: &ClientConfig) {
    let client = config.factory_env();
    let model = config
        .completion_model
        .unwrap_or_else(|| panic!("{} does not have the model set.", config.name));

    let Some(client) = client.as_completion() else {
        return;
    };

    let model = client.agent(model)
            .preamble("You are a calculator here to help the user perform arithmetic operations. Use the tools provided to answer the user's question.")
            .max_tokens(1024)
            .tool(Adder)
            .tool(Subtract)
            .build();

    let request = model.completion("Calculate 2 - 5", vec![]).await;

    assert!(
        request.is_ok(),
        "[{}]: Error occurred when building prompt, {}",
        config.name,
        request.err().unwrap()
    );

    let resp = request.unwrap().send().await;

    assert!(
        resp.is_ok(),
        "[{}]: Error occurred when prompting, {}",
        config.name,
        resp.err().unwrap()
    );

    let resp = resp.unwrap();

    assert!(
        resp.choice.iter().any(|content| match content {
            AssistantContent::ToolCall(tc) => {
                if tc.function.name != Subtract::NAME {
                    return false;
                }

                let arguments =
                    serde_json::from_value::<OperationArgs>((tc.function.arguments).clone())
                        .expect("Error parsing arguments");

                arguments.x == 2.0 && arguments.y == 5.0
            }
            _ => false,
        }),
        "[{}]: Model did not use the Subtract tool.",
        config.name
    )
}

#[tokio::test]
#[ignore]
async fn test_tools() {
    for p in providers().into_iter().filter(ClientConfig::is_env_var_set) {
        test_tools_client(&p).await;
    }
}

async fn test_streaming_client(config: &ClientConfig) {
    let client = config.factory_env();

    let Some(client) = client.as_completion() else {
        return;
    };

    let model = config
        .completion_model
        .unwrap_or_else(|| panic!("{} does not have the model set.", config.name));

    let model = client.completion_model(model);

    let resp = model.stream(CompletionRequest {
        preamble: None,
        tools: vec![],
        documents: vec![],
        temperature: None,
        max_tokens: None,
        additional_params: None,
        tool_choice: None,
        chat_history: OneOrMany::one(Message::user("What is the capital of France?")),
    });

    let mut resp = resp.await.unwrap();

    let mut received_chunk = false;

    while let Some(chunk) = resp.next().await {
        received_chunk = true;
        assert!(chunk.is_ok());
    }

    assert!(
        received_chunk,
        "[{}]: Failed to receive a chunk from stream",
        config.name
    );

    for choice in resp.choice {
        match choice {
            AssistantContent::Text(text) => {
                assert!(
                    text.text.to_lowercase().contains("paris"),
                    "[{}]: Did not answer with Paris",
                    config.name
                );
            }
            AssistantContent::ToolCall(_) => {}
            AssistantContent::Reasoning(_) => {}
        }
    }
}

#[tokio::test]
#[ignore]
async fn test_streaming() {
    for provider in providers().into_iter().filter(ClientConfig::is_env_var_set) {
        test_streaming_client(&provider).await;
    }
}

async fn test_streaming_tools_client(config: &ClientConfig) {
    let client = config.factory_env();
    let model = config
        .completion_model
        .unwrap_or_else(|| panic!("{} does not have the model set.", config.name));

    let Some(client) = client.as_completion() else {
        return;
    };

    let model = client.agent(model)
            .preamble("You are a calculator here to help the user perform arithmetic operations. Use the tools provided to answer the user's question.")
            .max_tokens(1024)
            .tool(Adder)
            .tool(Subtract)
            .build();

    let request = model.stream_completion("Calculate 2 - 5", vec![]).await;

    assert!(
        request.is_ok(),
        "[{}]: Error occurred when building prompt, {}",
        config.name,
        request.err().unwrap()
    );

    let resp = request.unwrap().stream().await;

    assert!(
        resp.is_ok(),
        "[{}]: Error occurred when prompting, {}",
        config.name,
        resp.err().unwrap()
    );

    let mut resp = resp.unwrap();

    let mut received_chunk = false;

    while let Some(chunk) = resp.next().await {
        received_chunk = true;
        assert!(chunk.is_ok());
    }

    assert!(
        received_chunk,
        "[{}]: Failed to receive a chunk from stream",
        config.name
    );

    assert!(
        resp.choice.iter().any(|content| match content {
            AssistantContent::ToolCall(tc) => {
                if tc.function.name != Subtract::NAME {
                    return false;
                }

                let arguments =
                    serde_json::from_value::<OperationArgs>((tc.function.arguments).clone())
                        .expect("Error parsing arguments");

                arguments.x == 2.0 && arguments.y == 5.0
            }
            _ => false,
        }),
        "[{}]: Model did not use the Subtract tool.",
        config.name
    )
}

#[tokio::test]
#[ignore]
async fn test_streaming_tools() {
    for p in providers().into_iter().filter(ClientConfig::is_env_var_set) {
        test_streaming_tools_client(&p).await;
    }
}

fn assert_feature<F, M>(
    name: &str,
    feature_name: &str,
    model_name: &str,
    feature: Option<F>,
    model: Option<M>,
) {
    assert_eq!(
        feature.is_some(),
        model.is_some(),
        "{} has{} implemented {} but config.{} is {}.",
        name,
        if feature.is_some() { "" } else { "n't" },
        feature_name,
        model_name,
        if model.is_some() { "some" } else { "none" }
    );
}

#[test]
#[ignore]
pub fn test_polymorphism() {
    for config in providers().into_iter().filter(ClientConfig::is_env_var_set) {
        let client = config.factory_env();
        assert_feature(
            config.name,
            "AsCompletion",
            "completion_model",
            client.as_completion(),
            config.completion_model,
        );

        assert_feature(
            config.name,
            "AsEmbeddings",
            "embeddings_model",
            client.as_embeddings(),
            config.embeddings_model,
        );
    }
}

async fn test_embed_client(config: &ClientConfig) {
    const TEST: &str = "Hello world.";

    let client = config.factory_env();

    let Some(client) = client.as_embeddings() else {
        return;
    };

    let model = config.embeddings_model.unwrap();

    let model = client.embedding_model(model);

    let resp = model.embed_text(TEST).await;

    assert!(
        resp.is_ok(),
        "[{}]: Error occurred when sending request, {}",
        config.name,
        resp.err().unwrap()
    );

    let resp = resp.unwrap();

    assert_eq!(resp.document, TEST);

    assert!(
        !resp.vec.is_empty(),
        "[{}]: Returned embed was empty",
        config.name
    );
}

#[tokio::test]
#[ignore]
async fn test_embed() {
    for config in providers().into_iter().filter(ClientConfig::is_env_var_set) {
        test_embed_client(&config).await;
    }
}

#[derive(Deserialize)]
struct OperationArgs {
    x: f32,
    y: f32,
}

#[derive(Debug, thiserror::Error)]
#[error("Math error")]
struct MathError;

#[derive(Deserialize, Serialize)]
struct Adder;
impl Tool for Adder {
    const NAME: &'static str = "add";

    type Error = MathError;
    type Args = OperationArgs;
    type Output = f32;

    async fn definition(&self, _prompt: String) -> ToolDefinition {
        ToolDefinition {
            name: "add".to_string(),
            description: "Add x and y together".to_string(),
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
                }
            }),
        }
    }

    async fn call(&self, args: Self::Args) -> anyhow::Result<Self::Output, Self::Error> {
        println!("[tool-call] Adding {} and {}", args.x, args.y);
        let result = args.x + args.y;
        Ok(result)
    }
}

#[derive(Deserialize, Serialize)]
struct Subtract;
impl Tool for Subtract {
    const NAME: &'static str = "subtract";

    type Error = MathError;
    type Args = OperationArgs;
    type Output = f32;

    async fn definition(&self, _prompt: String) -> ToolDefinition {
        serde_json::from_value(json!({
            "name": "subtract",
            "description": "Subtract y from x (i.e.: x - y)",
            "parameters": {
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
                }
            }
        }))
        .expect("Tool Definition")
    }

    async fn call(&self, args: Self::Args) -> anyhow::Result<Self::Output, Self::Error> {
        println!("[tool-call] Subtracting {} from {}", args.y, args.x);
        let result = args.x - args.y;
        Ok(result)
    }
}

use anyhow::Result;
use rig::prelude::*;
use rig::vector_store::in_memory_store::InMemoryVectorStore;

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    tracing_subscriber::fmt().init();

    // Create DeepSeek client
    let deepseek_client = rig_deepseek::client::Client::from_env();
    
    // Note: DeepSeek may not have embedding models, so this is a template
    // You would need to check if DeepSeek supports embeddings
    println!("DeepSeek Embedding Example");
    println!("========================");
    println!("Note: DeepSeek may not currently support embedding models.");
    println!("This is a template for when embedding support is added.");
    
    // This is commented out because DeepSeek may not support embeddings yet
    /*
    let embedding_model = deepseek_client.embedding_model("deepseek-embedding-model");
    
    // Sample documents to embed
    let documents = vec![
        "Rust is a systems programming language focused on safety and performance.",
        "DeepSeek is an AI company that develops large language models.",
        "Embeddings are numerical representations of text in a high-dimensional space.",
    ];

    // Embed documents
    let embeddings = embedding_model.embed_documents(&documents).await?;
    
    // Create an in-memory vector store
    let vector_store = InMemoryVectorStore::new(embedding_model);
    
    // Add documents to the vector store
    for (i, (document, embedding)) in documents.iter().zip(embeddings.iter()).enumerate() {
        vector_store.add_document(&format!("doc_{}", i), document, embedding).await?;
    }

    // Perform a similarity search
    let query = "What is DeepSeek?";
    let results = vector_store.search(query, 3).await?;
    
    println!("Top 3 similar documents:");
    for (i, (id, document, similarity)) in results.iter().enumerate() {
        println!("{}. ID: {}, Similarity: {:.4}", i + 1, id, similarity);
        println!("   Content: {}", document);
        println!();
    }
    */

    Ok(())
}
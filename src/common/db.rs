use mongodb::{Client, Database};

pub async fn connect(uri: &str, db_name: &str) -> anyhow::Result<Database> {
    let client = Client::with_uri_str(uri).await?;
    let db = client.database(db_name);
    tracing::info!("Connected to MongoDB: {}", db_name);
    Ok(db)
}

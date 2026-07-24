use sqlx::postgres::PgPoolOptions;
use dotenv::dotenv;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenv().ok();
    let database_url = std::env::var("DATABASE_URL").expect("DATABASE_URL must be set");
    let pool = PgPoolOptions::new()
        .max_connections(1)
        .connect(&database_url)
        .await?;

    let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM contracts")
        .fetch_one(&pool)
        .await?;

    println!("Total contracts in database: {}", count);
    
    let verified_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM contracts WHERE is_verified = true")
        .fetch_one(&pool)
        .await?;
    println!("Verified contracts: {}", verified_count);

    Ok(())
}

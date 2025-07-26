use kfleet::{create_router, AppState};
use log::info;
use sqlx::postgres::PgPoolOptions;
use std::sync::Arc;
use tera::Tera;
use tokio::net::TcpListener;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize logger
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info"))
        .format_timestamp(None)
        .init();
    
    info!("Starting fleet management system");

    // Load environment variables
    dotenvy::dotenv().ok();
    let db_url = std::env::var("DATABASE_URL")
        .expect("DATABASE_URL must be set in .env file");
    let port = std::env::var("PORT")
        .unwrap_or_else(|_| "3000".to_string())
        .parse::<u16>()?;
    let addr = format!("0.0.0.0:{port}");

    // Create database connection pool
    info!("Connecting to database...");
    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect(&db_url)
        .await?;
    info!("Database connection established");

    // Run database migrations
    info!("Running database migrations");
    sqlx::migrate!().run(&pool).await?;
    info!("Migrations completed");

    // Initialize template engine
    info!("Loading templates");
    let mut tera = match Tera::new("templates/**/*") {
        Ok(t) => t,
        Err(e) => {
            println!("Template parsing error: {}", e);
            return Err(e.into());
        }
    };
    tera.autoescape_on(vec!["html"]);
    info!("{} templates loaded", tera.templates.len());

    // Create shared application state
    let state = Arc::new(AppState {
        db: pool,
        templates: tera,
    });

    // Create router
    let app = create_router(Arc::clone(&state));

    // Create TCP listener
    info!("Binding to {}", addr);
    let listener = TcpListener::bind(&addr).await?;
    info!("Server listening on http://{}", addr);

    // Start server with axum::serve
    axum::serve(listener, app).await?;

    Ok(())
}


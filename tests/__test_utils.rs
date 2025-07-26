use axum_test::TestServer;
use kfleet::{
    app, 
    AppState,
    handlers::{staff, equipment, categories}
};
use sqlx::postgres::PgPoolOptions;
use tera::Tera;
use std::sync::Arc;

pub async fn setup_test_server() -> TestServer {
    // Use test database URL from environment
    dotenvy::from_filename(".env.test").ok();
    let db_url = std::env::var("TEST_DATABASE_URL")
        .expect("TEST_DATABASE_URL must be set in .env.test");
    
    // Create database pool
    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect(&db_url)
        .await
        .expect("Failed to connect to test database");
    
    // Run migrations
    sqlx::migrate!()
        .run(&pool)
        .await
        .expect("Failed to run migrations");
    
    // Initialize templates
    let mut tera = Tera::new("templates/**/*")
        .expect("Failed to create Tera instance");
    tera.autoescape_on(vec!["html"]);
    
    // Create application state
    let state = Arc::new(AppState {
        db: pool.clone(),
        templates: tera,
    });
    
    // Create test server
    TestServer::new(app(state).into_make_service())
        .expect("Failed to create test server")
}

pub async fn create_test_equipment(server: &TestServer) -> i32 {
    // Create test category
    let category = server.post("/categories")
        .form(&[("name", "Test Equipment Category")])
        .await;
    assert_eq!(category.status_code(), 303); // Redirect after create
    
    // Create test equipment
    let equipment = server.post("/equipment")
        .form(&[
            ("name", "Test Excavator"),
            ("brand", "CAT"),
            ("model", "320"),
            ("serial_number", "EXC-123"),
            ("acquisition_date", "2023-01-01T00:00:00Z"),
            ("category_id", "1"),
            ("status", "active"),
        ])
        .await;
    assert_eq!(equipment.status_code(), 303);
    
    1 // Return equipment ID
}

pub async fn create_test_staff(server: &TestServer, equipment_ids: &[i32]) -> i32 {
    let mut form = vec![
        ("full_name", "Test Operator"),
        ("license_number", "OP-123"),
        ("contact_info", "test@example.com"),
    ];
    
    for id in equipment_ids {
        form.push(("assigned_equipment", &id.to_string()));
    }
    
    let response = server.post("/staff")
        .form(&form)
        .await;
    
    assert_eq!(response.status_code(), 303);
    
    // Extract staff ID from redirect location
    let location = response.headers().get("location").unwrap().to_str().unwrap();
    location.split('/').last().unwrap().parse().unwrap()
}


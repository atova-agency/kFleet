use axum_test::TestServer;
use insta::{assert_html_snapshot, with_settings};
use kfleet::{create_router, AppState};
use sqlx::{Pool, Postgres, PgPoolOptions};
use tera::Tera;
use std::sync::Arc;

async fn snapshot_server() -> TestServer {
    let db_url = "postgres://snapshot:snapshot@localhost:5432/kfleet_snapshot";
    let pool = PgPoolOptions::new()
        .max_connections(1)
        .connect(db_url)
        .await
        .unwrap();
    
    sqlx::migrate!().run(&pool).await.unwrap();
    
    // Create consistent test data
    sqlx::query!("INSERT INTO categories (name) VALUES ('Heavy Equipment')")
        .execute(&pool)
        .await
        .unwrap();
    
    sqlx::query!(
        "INSERT INTO equipment (name, brand, model, category_id, current_status) 
        VALUES ($1, $2, $3, $4, $5)",
        "Excavator", "CAT", "320", 1, "active"
    )
    .execute(&pool)
    .await
    .unwrap();
    
    let state = Arc::new(AppState {
        db: pool,
        templates: Tera::new("templates/**/*").unwrap(),
    });
    
    TestServer::new(create_router(state)).unwrap()
}

#[tokio::test]
async fn test_new_staff_form_snapshot() {
    let server = snapshot_server().await;
    let response = server.get("/staff/new").await;
    
    with_settings!({
        filters => vec![
            (r"value=\"\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}Z\"", "value=\"[TIMESTAMP]\""),
            (r"csrf_token=[^\"]+", "csrf_token=[TOKEN]")
        ]
    }, {
        assert_html_snapshot!("new_staff_form", response.text());
    });
}

#[tokio::test]
async fn test_edit_staff_form_snapshot() {
    let server = snapshot_server().await;
    
    // Create staff with assignments
    let staff_id = {
        let response = server.post("/staff")
            .form(&[
                ("full_name", "John Doe"),
                ("assigned_equipment", "1"),
            ])
            .await;
        let location = response.headers().get("location").unwrap().to_str().unwrap();
        location.split('/').last().unwrap().parse().unwrap()
    };
    
    let response = server.get(&format!("/staff/{}/edit", staff_id)).await;
    
    with_settings!({
        filters => vec![
            (r"value=\"\d+\"", "value=\"[ID]\""),
            (r"John Doe", "[STAFF_NAME]")
        ]
    }, {
        assert_html_snapshot!("edit_staff_form", response.text());
    });
}


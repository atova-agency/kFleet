use kfleet::handlers::staff;
use kfleet::AppState;
use axum::extract::Extension;
use sqlx::{Pool, Postgres};
use std::sync::Arc;

// Mock database for handler tests
async fn mock_db() -> Pool<Postgres> {
    let db_url = "postgres://mock:mock@localhost:5432/mock";
    PgPoolOptions::new()
        .max_connections(1)
        .connect(db_url)
        .await
        .unwrap()
}

#[tokio::test]
async fn test_list_staff_handler() {
    let db = mock_db().await;
    let state = Arc::new(AppState {
        db,
        templates: Tera::default(),
    });
    
    let result = staff::list(Extension(state)).await;
    assert!(result.is_ok());
    let html = result.unwrap().0;
    assert!(html.contains("Staff Members"));
}

#[tokio::test]
async fn test_new_form_handler() {
    let db = mock_db().await;
    let state = Arc::new(AppState {
        db,
        templates: Tera::default(),
    });
    
    let result = staff::new_form(Extension(state)).await;
    assert!(result.is_ok());
    let html = result.unwrap().0;
    assert!(html.contains("Add New Staff"));
}

#[tokio::test]
async fn test_update_equipment_assignments() {
    let mut db = mock_db().await;
    let mut tx = db.begin().await.unwrap();
    
    let result = staff::update_equipment_assignments(
        &mut tx,
        1,
        &[1, 2, 3]
    ).await;
    
    assert!(result.is_ok());
    
    // Verify assignments were created
    let assignments = sqlx::query!(
        "SELECT equipment_id FROM equipment_operator WHERE operator_id = $1",
        1
    )
    .fetch_all(&mut tx)
    .await
    .unwrap();
    
    assert_eq!(assignments.len(), 3);
}


use axum_test::{TestServer, TestResponse};
use kfleet::{create_router, AppState};
use sqlx::{Pool, Postgres, PgPoolOptions};
use tera::Tera;
use std::sync::Arc;

async fn test_server() -> TestServer {
    let db_url = "postgres://test:test@localhost:5432/kfleet_test";
    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect(db_url)
        .await
        .unwrap();
    
    // Run migrations
    sqlx::migrate!().run(&pool).await.unwrap();
    
    let state = Arc::new(AppState {
        db: pool,
        templates: Tera::default(),
    });
    
    TestServer::new(create_router(state)).unwrap()
}

#[tokio::test]
async fn test_create_staff_with_assignments() {
    let server = test_server().await;
    
    // Create test equipment
    let equipment = server.post("/equipment")
        .form(&[
            ("name", "Excavator"),
            ("brand", "CAT"),
            ("model", "320"),
            ("serial_number", "EXC-123"),
            ("acquisition_date", "2023-01-01T00:00:00Z"),
            ("category_id", "1"),
            ("status", "active"),
        ])
        .await;
    assert_eq!(equipment.status(), 303);
    
    // Create staff with assignments
    let response = server.post("/staff")
        .form(&[
            ("full_name", "John Doe"),
            ("contact_info", "john@example.com"),
            ("license_number", "OP-123"),
            ("assigned_equipment", "1"),  // Assign equipment ID 1
        ])
        .await;
    
    assert_eq!(response.status(), 303);
    
    // Verify staff creation
    let staff = sqlx::query!("SELECT * FROM staff WHERE full_name = 'John Doe'")
        .fetch_one(&server.app_state().db)
        .await
        .unwrap();
    
    // Verify assignments
    let assignments = sqlx::query!(
        "SELECT equipment_id FROM equipment_operator WHERE operator_id = $1",
        staff.id
    )
    .fetch_all(&server.app_state().db)
    .await
    .unwrap();
    
    assert_eq!(assignments.len(), 1);
    assert_eq!(assignments[0].equipment_id, 1);
}

#[tokio::test]
async fn test_edit_staff_assignments() {
    let server = test_server().await;
    
    // Create test equipment
    for i in 1..=3 {
        server.post("/equipment")
            .form(&[
                ("name", &format!("Equipment {}", i)),
                // ... other fields
            ])
            .await;
    }
    
    // Create staff with initial assignments
    let staff_id = create_test_staff(&server, &[1]).await;
    
    // Update assignments
    let response = server.post(&format!("/staff/{}", staff_id))
        .form(&[
            ("full_name", "Updated Name"),
            ("assigned_equipment", "2"),
            ("assigned_equipment", "3"),
        ])
        .await;
    
    assert_eq!(response.status(), 303);
    
    // Verify updated assignments
    let assignments = sqlx::query!(
        "SELECT equipment_id FROM equipment_operator WHERE operator_id = $1",
        staff_id
    )
    .fetch_all(&server.app_state().db)
    .await
    .unwrap();
    
    assert_eq!(assignments.len(), 2);
    assert_eq!(assignments[0].equipment_id, 2);
    assert_eq!(assignments[1].equipment_id, 3);
}

async fn create_test_staff(server: &TestServer, equipment_ids: &[i32]) -> i32 {
    let mut form = vec![
        ("full_name", "Test Operator"),
        ("license_number", "OP-123"),
    ];
    
    for id in equipment_ids {
        form.push(("assigned_equipment", &id.to_string()));
    }
    
    let response = server.post("/staff")
        .form(&form)
        .await;
    
    assert_eq!(response.status(), 303);
    
    // Extract staff ID from redirect
    let location = response.headers().get("location").unwrap().to_str().unwrap();
    location.split('/').last().unwrap().parse().unwrap()
}


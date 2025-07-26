use axum::{
    extract::Extension,
    http::StatusCode,
    routing::{get, post},
    Router,
};
//use axum::response::Html;
use serde::Serialize;
use sqlx::postgres::PgPool;
use sqlx::FromRow;
use std::sync::Arc;
use tera::Tera;

pub mod handlers {
    pub mod categories;
    pub mod equipment;
    pub mod staff;
}

// Shared application state
#[derive(Clone)]
pub struct AppState {
    pub db: PgPool,
    pub templates: Tera,
}

// Define serializable structs
#[derive(Serialize, FromRow)]
pub struct StatusCounts {
    pub active: i64,
    pub maintenance: i64,
    pub retired: i64,
}

#[derive(Debug, Serialize, FromRow)]
pub struct MaintenanceAlert {
    pub name: String,
    pub next_maintenance: Option<chrono::DateTime<chrono::Utc>>,
}

#[derive(Debug, Serialize, FromRow)]
pub struct InsuranceAlert {
    pub name: String,
    pub insurance_renewal: Option<chrono::DateTime<chrono::Utc>>,
}

#[derive(Debug, Serialize, FromRow)]
pub struct RecentEquipment {
    pub name: String,
    pub status: String,
    pub acquisition_date: chrono::DateTime<chrono::Utc>,
    pub next_maintenance: Option<chrono::DateTime<chrono::Utc>>,
    pub category_name: String,
}

#[derive(Debug, Serialize, FromRow)]
pub struct RecentMaintenance {
    pub maintenance_date: chrono::DateTime<chrono::Utc>,
    pub equipment_name: String,
    pub description: String,
    pub cost: Option<f64>,
}

pub fn create_router(state: Arc<AppState>) -> Router {
    Router::new()
        // Category routes
        .route("/categories", get(handlers::categories::list)
                             .post(handlers::categories::create))
        .route("/categories/new", get(handlers::categories::new_form))
        .route("/categories/{id}/edit", get(handlers::categories::edit_form))
        .route("/categories/{id}", post(handlers::categories::update))
        .route("/categories/{id}/delete", post(handlers::categories::delete))
        
        // Equipment routes
        .route("/equipment", get(handlers::equipment::list)
                            .post(handlers::equipment::create))
        .route("/equipment/new", get(handlers::equipment::new_form))
        .route("/equipment/{id}/edit", get(handlers::equipment::edit_form))
        .route("/equipment/{id}", post(handlers::equipment::update))
        .route("/equipment/{id}/delete", post(handlers::equipment::delete))
        
        // Staff routes
        .route("/staff", get(handlers::staff::list)
                        .post(handlers::staff::create))
        .route("/staff/new", get(handlers::staff::new_form))
        .route("/staff/{id}/edit", get(handlers::staff::edit_form))
        .route("/staff/{id}", post(handlers::staff::update))
        .route("/staff/{id}/delete", post(handlers::staff::delete))

        // Mobile App
        .route("/app", get(mobile))
        
        // Dashboard
        .route("/", get(dashboard))
        .layer(Extension(state))
        .fallback(not_found)
}


pub async fn mobile(
    Extension(state): Extension<Arc<AppState>>,
) -> Result<axum::response::Html<String>, String> {
    log::info!("serving mobile");

    let mut ctx = tera::Context::new();
    //ctx.insert("app", &task);
    state.templates.render("app.html", &ctx)
        .map_err(|e| e.to_string())
        .map(axum::response::Html)
}

/* Business Logic: Dashboard shows critical maintenance deadlines */
pub async fn dashboard(
    Extension(state): Extension<Arc<AppState>>
) -> Result<axum::response::Html<String>, (axum::http::StatusCode, String)> {
    log::info!("Serving dashboard");
    
    // Fetch status counts (using COALESCE to ensure non-null)
    let status_counts = sqlx::query_as!(
        StatusCounts,
        r#"SELECT
            COALESCE(COUNT(*) FILTER (WHERE current_status = 'active'), 0) as "active!",
            COALESCE(COUNT(*) FILTER (WHERE current_status = 'maintenance'), 0) as "maintenance!",
            COALESCE(COUNT(*) FILTER (WHERE current_status = 'retired'), 0) as "retired!"
        FROM equipment"#
    )
    .fetch_one(&state.db)
    .await
    .map_err(|err| {
        log::error!("Failed to fetch status counts: {}", err);
        (StatusCode::INTERNAL_SERVER_ERROR, "Database error".to_string())
    })?;

    // Fetch upcoming maintenance (next 30 days)
    let maintenance_alerts = sqlx::query_as!(
        MaintenanceAlert,
        r#"SELECT name, next_maintenance 
        FROM equipment 
        WHERE next_maintenance BETWEEN CURRENT_DATE AND CURRENT_DATE + INTERVAL '30 days'
            AND current_status != 'retired'
        ORDER BY next_maintenance
        LIMIT 5"#
    )
    .fetch_all(&state.db)
    .await
    .map_err(|err| {
        log::error!("Failed to fetch maintenance alerts: {}", err);
        (StatusCode::INTERNAL_SERVER_ERROR, "Database error".to_string())
    })?;

    // Fetch insurance renewals (next 30 days)
    let insurance_alerts = sqlx::query_as!(
        InsuranceAlert,
        r#"SELECT name, insurance_renewal 
        FROM equipment 
        WHERE insurance_renewal BETWEEN CURRENT_DATE AND CURRENT_DATE + INTERVAL '30 days'
            AND current_status != 'retired'
        ORDER BY insurance_renewal
        LIMIT 5"#
    )
    .fetch_all(&state.db)
    .await
    .map_err(|err| {
        log::error!("Failed to fetch insurance alerts: {}", err);
        (StatusCode::INTERNAL_SERVER_ERROR, "Database error".to_string())
    })?;

    // Fetch recent equipment
    let recent_equipment = sqlx::query_as!(
        RecentEquipment,
        r#"SELECT 
            e.name, 
            e.current_status as "status!", 
            e.acquisition_date as "acquisition_date!", 
            e.next_maintenance,
            c.name as "category_name!"
        FROM equipment e
        JOIN categories c ON e.category_id = c.id
        ORDER BY e.created_at DESC
        LIMIT 6"#
    )
    .fetch_all(&state.db)
    .await
    .map_err(|err| {
        log::error!("Failed to fetch recent equipment: {}", err);
        (StatusCode::INTERNAL_SERVER_ERROR, "Database error".to_string())
    })?;

    // Fetch recent maintenance records
    let recent_maintenance = sqlx::query_as!(
        RecentMaintenance,
        r#"SELECT 
            m.maintenance_date as "maintenance_date!", 
            e.name as "equipment_name!", 
            m.description as "description!", 
            m.cost
        FROM maintenance_history m
        JOIN equipment e ON m.equipment_id = e.id
        ORDER BY m.maintenance_date DESC
        LIMIT 5"#
    )
    .fetch_all(&state.db)
    .await
    .map_err(|err| {
        log::error!("Failed to fetch recent maintenance: {}", err);
        (StatusCode::INTERNAL_SERVER_ERROR, "Database error".to_string())
    })?;

    // Prepare template context
    let mut context = tera::Context::new();
    context.insert("status_counts", &status_counts);
    context.insert("maintenance_alerts", &maintenance_alerts);
    context.insert("insurance_alerts", &insurance_alerts);
    context.insert("recent_equipment", &recent_equipment);
    context.insert("recent_maintenance", &recent_maintenance);

    // Render template
    match state.templates.render("index.html", &context) {
        Ok(content) => Ok(axum::response::Html(content)),
        Err(err) => {
            log::error!("Dashboard template error: {}", err);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                "Failed to render dashboard".to_string(),
            ))
        }
    }
}

// 404 handler
pub async fn not_found() -> (StatusCode, &'static str) {
    log::warn!("404 - Page not found");
    (StatusCode::NOT_FOUND, "Page not found")
}

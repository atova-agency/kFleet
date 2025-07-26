use crate::AppState;
use axum::{
    extract::{Extension, Form, Path},
    response::{Html, Redirect},
};
//use chrono::{DateTime, Utc};
use chrono::{DateTime, FixedOffset, NaiveDateTime, TimeZone, Utc};
use log::{error, info, warn};
use serde::Deserialize;
use serde::Serialize;
use sqlx::{FromRow, PgPool};
use std::sync::Arc;

#[derive(Debug, FromRow, Serialize)]
pub struct Equipment {
    pub id: i32,
    pub name: String,
    pub brand: String,
    pub model: String,
    pub serial_number: String,
    pub acquisition_date: DateTime<Utc>,
    pub category_id: i32,
    pub category_name: String,
    pub insurance_renewal: Option<DateTime<Utc>>,
    pub next_maintenance: Option<DateTime<Utc>>,
    pub fuel_capacity: Option<f64>,
    pub status: String,
}

#[derive(Debug, FromRow, Serialize)]
pub struct EquipmentShort {
    pub id: i32,
    pub name: String,
    pub brand: String,
    pub model: String,
    pub status: String,
}

#[derive(Debug, FromRow, Serialize)]
pub struct Category {
    pub id: i32,
    pub name: String,
}

#[derive(Debug, Deserialize)]
pub struct EquipmentForm {
    pub name: String,
    pub brand: String,
    pub model: String,
    pub serial_number: String,
    pub acquisition_date: String,
    pub category_id: i32,
    pub insurance_renewal: Option<String>,
    pub next_maintenance: Option<String>,
    pub fuel_capacity: Option<f64>,
    pub status: String,
    pub timezone_offset: Option<i32>,  
}

// CREATE
pub async fn create(
    Extension(state): Extension<Arc<AppState>>,
    Form(form): Form<EquipmentForm>,
) -> Result<Redirect, String> {
    info!("Creating new equipment: {}", form.name);
    
    // Get timezone offset from form (default to UTC)
    let tz_offset = form.timezone_offset.unwrap_or(0);
    
    let acquisition_date = parse_timestamptz(&form.acquisition_date, tz_offset)?;
    let insurance_renewal = parse_optional_timestamptz(form.insurance_renewal, tz_offset)?;
    let next_maintenance = parse_optional_timestamptz(form.next_maintenance, tz_offset)?;

    sqlx::query!(
        r#"
        INSERT INTO equipment (
            name, brand, model, serial_number, acquisition_date,
            category_id, insurance_renewal,
            next_maintenance, fuel_capacity, current_status
        ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)
        "#,
        form.name,
        form.brand,
        form.model,
        form.serial_number,
        acquisition_date,
        form.category_id,
        insurance_renewal,
        next_maintenance,
        form.fuel_capacity,
        form.status
    )
    .execute(&state.db)
    .await
    .map_err(|e| {
        error!("Equipment creation failed: {}", e);
        e.to_string()
    })?;

    info!("Equipment '{}' created successfully", form.name);
    Ok(Redirect::to("/equipment"))
}


// LIST
pub async fn list(
    Extension(state): Extension<Arc<AppState>>,
) -> Result<Html<String>, String> {
    info!("Listing equipment");
    
    let equipment = sqlx::query_as!(
        Equipment,
        r#"
        SELECT 
            e.id, e.name, e.brand, e.model, e.serial_number, 
            e.acquisition_date, e.category_id, c.name as category_name,
            e.insurance_renewal, e.next_maintenance, e.fuel_capacity,
            e.current_status as "status!"
        FROM equipment e
        JOIN categories c ON e.category_id = c.id
        ORDER BY e.name
        "#
    )
    .fetch_all(&state.db)
    .await
    .map_err(|e| {
        error!("Failed to fetch equipment: {}", e);
        e.to_string()
    })?;

    let categories = get_categories(&state.db).await?;

    let mut ctx = tera::Context::new();
    ctx.insert("equipment", &equipment);
    ctx.insert("categories", &categories);
    state.templates.render("equipment/index.html", &ctx)
        .map_err(|e| e.to_string())
        .map(Html)
}

// NEW FORM
pub async fn new_form(
    Extension(state): Extension<Arc<AppState>>,
) -> Result<Html<String>, String> {
    info!("Serving new equipment form");
    
    let categories = get_categories(&state.db).await?;

    // Create a default equipment struct for the form
    #[derive(serde::Serialize)]
    struct EquipmentDefault {
        acquisition_date: Option<String>,
        insurance_renewal: Option<String>,
        next_maintenance: Option<String>,
    }
    
    let equipment = EquipmentDefault {
        acquisition_date: None,
        insurance_renewal: None,
        next_maintenance: None,
    };

    let mut ctx = tera::Context::new();
    ctx.insert("categories", &categories);
    ctx.insert("equipment", &equipment);  // Pass equipment to template
    state.templates.render("equipment/new.html", &ctx)
        .map_err(|e| e.to_string())
        .map(Html)
}

// EDIT FORM
pub async fn edit_form(
    Path(id): Path<i32>,
    Extension(state): Extension<Arc<AppState>>,
) -> Result<Html<String>, String> {
    info!("Editing equipment ID: {}", id);
    
    // Reuse existing Equipment struct
    let equipment = sqlx::query_as!(
        Equipment,
        r#"
        SELECT 
            e.id, e.name, e.brand, e.model, e.serial_number, 
            e.acquisition_date, e.category_id, c.name as category_name,
            e.insurance_renewal, e.next_maintenance, e.fuel_capacity,
            e.current_status as "status!"
        FROM equipment e
        JOIN categories c ON e.category_id = c.id
        WHERE e.id = $1
        "#,
        id
    )
    .fetch_one(&state.db)
    .await
    .map_err(|e| {
        warn!("Equipment {} not found: {}", id, e);
        e.to_string()
    })?;

    let categories = get_categories(&state.db).await?;

    let mut ctx = tera::Context::new();
    ctx.insert("equipment", &equipment);
    ctx.insert("categories", &categories);
    state.templates.render("equipment/edit.html", &ctx)
        .map_err(|e| e.to_string())
        .map(Html)
}

// UPDATE
pub async fn update(
    Path(id): Path<i32>,
    Extension(state): Extension<Arc<AppState>>,
    Form(form): Form<EquipmentForm>,
) -> Result<Redirect, String> {
    info!("Updating equipment ID: {}", id);
    
    // Get timezone offset from form (default to UTC)
    let tz_offset = form.timezone_offset.unwrap_or(0);
    
    let acquisition_date = parse_timestamptz(&form.acquisition_date, tz_offset)?;
    let insurance_renewal = parse_optional_timestamptz(form.insurance_renewal, tz_offset)?;
    let next_maintenance = parse_optional_timestamptz(form.next_maintenance, tz_offset)?;

    sqlx::query!(
        r#"
        UPDATE equipment SET
            name = $1,
            brand = $2,
            model = $3,
            serial_number = $4,
            acquisition_date = $5,
            category_id = $6,
            insurance_renewal = $7,
            next_maintenance = $8,
            fuel_capacity = $9,
            current_status = $10
        WHERE id = $11
        "#,
        form.name,
        form.brand,
        form.model,
        form.serial_number,
        acquisition_date,
        form.category_id,
        insurance_renewal,
        next_maintenance,
        form.fuel_capacity,
        form.status,
        id
    )
    .execute(&state.db)
    .await
    .map_err(|e| {
        error!("Equipment update failed: {}", e);
        e.to_string()
    })?;

    info!("Equipment {} updated successfully", id);
    Ok(Redirect::to("/equipment"))
}

// UPDATED DATE PARSING FUNCTIONS
fn parse_timestamptz(datetime_str: &str, tz_offset: i32) -> Result<DateTime<Utc>, String> {
    if datetime_str.is_empty() {
        return Err("Empty datetime string".to_string());
    }
    
    let naive_dt = NaiveDateTime::parse_from_str(datetime_str, "%Y-%m-%dT%H:%M")
        .map_err(|e| format!("Invalid datetime format: {}", e))?;
    
    let offset_seconds = tz_offset * 3600;
    let offset = FixedOffset::east_opt(offset_seconds)
        .ok_or("Invalid timezone offset")?;
    
    Ok(offset
        .from_local_datetime(&naive_dt)
        .single()
        .ok_or("Ambiguous datetime")?
        .with_timezone(&Utc))
}

// Handle both None and empty strings
fn parse_optional_timestamptz(
    datetime_opt: Option<String>,
    tz_offset: i32,
) -> Result<Option<DateTime<Utc>>, String> {
    match datetime_opt {
        // Handle empty strings as None
        Some(s) if s.trim().is_empty() => Ok(None),
        Some(s) => parse_timestamptz(&s, tz_offset).map(Some),
        None => Ok(None),
    }
}

// DELETE
pub async fn delete(
    Path(id): Path<i32>,
    Extension(state): Extension<Arc<AppState>>,
) -> Result<Redirect, String> {
    info!("Deleting equipment ID: {}", id);
    
    sqlx::query!(
        "DELETE FROM equipment WHERE id = $1",
        id
    )
    .execute(&state.db)
    .await
    .map_err(|e| {
        error!("Equipment deletion failed: {}", e);
        e.to_string()
    })?;

    info!("Equipment {} deleted", id);
    Ok(Redirect::to("/equipment"))
}

// Helper functions
async fn get_categories(pool: &PgPool) -> Result<Vec<Category>, String> {
    sqlx::query_as!(
        Category,
        "SELECT id, name FROM categories ORDER BY name"
    )
    .fetch_all(pool)
    .await
    .map_err(|e| e.to_string())
}


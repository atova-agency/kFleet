use crate::AppState;
use axum::{
    extract::{Extension, Path},
    response::{Html, Redirect},
};
use axum_extra::extract::Form; // Use axum_extra's Form
use log::{info, warn};
use serde::Deserialize;
use serde::Serialize;
use sqlx::{FromRow, PgPool};
use std::sync::Arc;

#[derive(Debug, FromRow, Serialize)]
pub struct Staff {
    pub id: i32,
    pub full_name: String,
    pub contact_info: Option<String>,
    pub license_number: Option<String>,
}

#[derive(Debug, FromRow, Serialize)]
pub struct EquipmentShort {
    pub id: i32,
    pub name: String,
    pub brand: String,
    pub model: String,
    pub status: String,
}

#[derive(Debug, Deserialize)]
pub struct StaffForm {
    pub full_name: String,
    pub contact_info: Option<String>,
    pub license_number: Option<String>,
    #[serde(default)]
    pub assigned_equipment: Vec<i32>,
}

#[derive(Debug, Serialize)]
pub struct StaffList {
    pub id: i32,
    pub full_name: String,
    pub contact_info: Option<String>,
    pub license_number: Option<String>,
    pub equipment_names: Vec<String>,
}

// CREATE
pub async fn create(
    Extension(state): Extension<Arc<AppState>>,
    Form(form): Form<StaffForm>,
) -> Result<Redirect, String> {
    info!("Creating new staff: {}", form.full_name);
    
    let mut tx = state.db.begin().await.map_err(|e| e.to_string())?;
    
    let staff_id = sqlx::query!(
        r#"
        INSERT INTO staff (full_name, contact_info, license_number)
        VALUES ($1, $2, $3)
        RETURNING id
        "#,
        form.full_name,
        form.contact_info,
        form.license_number,
    )
    .fetch_one(&mut *tx)
    .await
    .map_err(|e| {
        warn!("Staff creation failed: {}", e);
        e.to_string()
    })?
    .id;
    
    update_equipment_assignments(&mut tx, staff_id, &form.assigned_equipment).await?;

    tx.commit().await.map_err(|e| e.to_string())?;

    info!("Staff '{}' created successfully", form.full_name);
    Ok(Redirect::to("/staff"))
}

// LIST
pub async fn list(
    Extension(state): Extension<Arc<AppState>>,
) -> Result<Html<String>, String> {
    info!("Listing staff");
    
    let staff = sqlx::query!(
        r#"
        SELECT 
            s.id, 
            s.full_name, 
            s.contact_info as "contact_info?",
            s.license_number as "license_number?",
            COALESCE(
                ARRAY_AGG(e.name) FILTER (WHERE e.name IS NOT NULL), 
                ARRAY[]::text[]
            ) as "equipment_names!: Vec<String>"
        FROM staff s
        LEFT JOIN equipment_operator eo ON eo.operator_id = s.id
        LEFT JOIN equipment e ON eo.equipment_id = e.id
        GROUP BY s.id
        ORDER BY s.full_name
        "#
    )
    .map(|row| StaffList {
        id: row.id,
        full_name: row.full_name,
        contact_info: row.contact_info,
        license_number: row.license_number,
        equipment_names: row.equipment_names,
    })
    .fetch_all(&state.db)
    .await
    .map_err(|e| {
        warn!("Failed to fetch staff: {}", e);
        e.to_string()
    })?;

    let mut ctx = tera::Context::new();
    ctx.insert("staff", &staff);
    state.templates.render("staff/index.html", &ctx)
        .map_err(|e| e.to_string())
        .map(Html)
}

// NEW FORM
pub async fn new_form(
    Extension(state): Extension<Arc<AppState>>,
) -> Result<Html<String>, String> {
    info!("Serving new staff form");
        
    let equipment = sqlx::query_as!(
        EquipmentShort,
        "SELECT id, name, brand, model, current_status as \"status!\" FROM equipment ORDER BY name"
    )
    .fetch_all(&state.db)
    .await
    .map_err(|e| e.to_string())?;

    // Create an empty vector for assigned equipment IDs
    let assigned_equipment_ids: Vec<i32> = Vec::new();

    let mut ctx = tera::Context::new();
    ctx.insert("equipment", &equipment);
    ctx.insert("assigned_equipment_ids", &assigned_equipment_ids);  // Add this line
    
    state.templates.render("staff/new.html", &ctx)
        .map_err(|e| e.to_string())
        .map(Html)
}

// EDIT FORM
pub async fn edit_form(
    Path(id): Path<i32>,
    Extension(state): Extension<Arc<AppState>>,
) -> Result<Html<String>, String> {
    info!("Editing staff ID: {}", id);
    
    #[derive(Serialize)]
    struct StaffRecord {
        id: i32,
        full_name: String,
        contact_info: Option<String>,
        license_number: Option<String>,
    }
    
    let staff = sqlx::query_as!(
        StaffRecord,
        "SELECT id, full_name, contact_info, license_number FROM staff WHERE id = $1",
        id
    )
    .fetch_one(&state.db)
    .await
    .map_err(|e| {
        warn!("Staff {} not found: {}", id, e);
        e.to_string()
    })?;

    let equipment = sqlx::query_as!(
        EquipmentShort,
        "SELECT id, name, brand, model, current_status as \"status!\" FROM equipment ORDER BY name"
    )
    .fetch_all(&state.db)
    .await
    .map_err(|e| e.to_string())?;

    let assigned_equipment = sqlx::query!(
        "SELECT equipment_id FROM equipment_operator WHERE operator_id = $1",
        id
    )
    .fetch_all(&state.db)
    .await
    .map_err(|e| e.to_string())?
    .into_iter()
    .map(|r| r.equipment_id)
    .collect::<Vec<_>>();

    let mut ctx = tera::Context::new();
    ctx.insert("staff", &staff);
    ctx.insert("equipment", &equipment);
    ctx.insert("assigned_equipment_ids", &assigned_equipment);
    state.templates.render("staff/edit.html", &ctx)
        .map_err(|e| e.to_string())
        .map(Html)
}

// UPDATE
pub async fn update(
    Path(id): Path<i32>,
    Extension(state): Extension<Arc<AppState>>,
    Form(form): Form<StaffForm>,
) -> Result<Redirect, String> {
    info!("Updating staff ID: {}", id);
    
    let mut tx = state.db.begin().await.map_err(|e| e.to_string())?;
    
    sqlx::query!(
        r#"
        UPDATE staff SET
            full_name = $1,
            contact_info = $2,
            license_number = $3
        WHERE id = $4
        "#,
        form.full_name,
        form.contact_info,
        form.license_number,
        id
    )
    .execute(&mut *tx)
    .await
    .map_err(|e| {
        warn!("Staff update failed: {}", e);
        e.to_string()
    })?;
    
    update_equipment_assignments(&mut tx, id, &form.assigned_equipment).await?;

    tx.commit().await.map_err(|e| e.to_string())?;

    info!("Staff {} updated successfully", id);
    Ok(Redirect::to("/staff"))
}

// DELETE
pub async fn delete(
    Path(id): Path<i32>,
    Extension(state): Extension<Arc<AppState>>,
) -> Result<Redirect, String> {
    info!("Deleting staff ID: {}", id);
    
    let assignment_count: i64 = sqlx::query_scalar!(
        "SELECT COUNT(*) FROM equipment_operator WHERE operator_id = $1",
        id
    )
    .fetch_one(&state.db)
    .await
    .map_err(|e| e.to_string())?
    .unwrap_or(0);
    
    if assignment_count > 0 {
        warn!("Cannot delete staff {} with {} equipment assignments", id, assignment_count);
        return Err(format!("Staff is assigned to {} equipment items", assignment_count));
    }
    
    sqlx::query!(
        "DELETE FROM staff WHERE id = $1",
        id
    )
    .execute(&state.db)
    .await
    .map_err(|e| {
        warn!("Staff deletion failed: {}", e);
        e.to_string()
    })?;

    info!("Staff {} deleted", id);
    Ok(Redirect::to("/staff"))
}

// Helper functions
async fn update_equipment_assignments(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    staff_id: i32,
    equipment_ids: &[i32],
) -> Result<(), String> {
    sqlx::query!(
        "DELETE FROM equipment_operator WHERE operator_id = $1",
        staff_id
    )
    .execute(&mut **tx)
    .await
    .map_err(|e| e.to_string())?;
    
    for equipment_id in equipment_ids {
        sqlx::query!(
            "INSERT INTO equipment_operator (operator_id, equipment_id) VALUES ($1, $2)",
            staff_id,
            equipment_id
        )
        .execute(&mut **tx)
        .await
        .map_err(|e| e.to_string())?;
    }
    
    Ok(())
}


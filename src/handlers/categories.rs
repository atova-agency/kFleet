use crate::AppState;
use axum::{
    extract::{Extension, Form, Path},
    response::{Html, Redirect},
};
use log::{info, warn};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use std::sync::Arc;

#[derive(Debug, FromRow, Serialize)]
pub struct Category {
    pub id: i32,
    pub name: String,
    pub equipment_count: i64,
}

#[derive(Debug, Deserialize)]
pub struct CategoryForm {
    pub name: String,
}

// CREATE
pub async fn create(
    Extension(state): Extension<Arc<AppState>>,
    Form(form): Form<CategoryForm>,
) -> Result<Redirect, String> {
    info!("Creating new category: {}", form.name);
    
    sqlx::query!(
        "INSERT INTO categories (name) VALUES ($1)",
        form.name
    )
    .execute(&state.db)
    .await
    .map_err(|e| {
        warn!("Category creation failed: {}", e);
        e.to_string()
    })?;

    info!("Category '{}' created successfully", form.name);
    Ok(Redirect::to("/categories"))
}

// LIST
pub async fn list(
    Extension(state): Extension<Arc<AppState>>,
) -> Result<Html<String>, String> {
    info!("Listing categories");
    
    let categories = sqlx::query_as!(
        Category,
        r#"
        SELECT 
            c.id, 
            c.name, 
            COUNT(e.id) as "equipment_count!: i64"
        FROM categories c
        LEFT JOIN equipment e ON e.category_id = c.id
        GROUP BY c.id, c.name
        ORDER BY c.name
        "#
    )
    .fetch_all(&state.db)
    .await
    .map_err(|e| {
        warn!("Failed to fetch categories: {}", e);
        e.to_string()
    })?;

    let mut ctx = tera::Context::new();
    ctx.insert("categories", &categories);
    state.templates.render("categories/index.html", &ctx)
        .map_err(|e| e.to_string())
        .map(Html)
}

// NEW FORM
pub async fn new_form(
    Extension(state): Extension<Arc<AppState>>,
) -> Result<Html<String>, String> {
    info!("Serving new category form");
    state.templates.render("categories/new.html", &tera::Context::new())
        .map_err(|e| e.to_string())
        .map(Html)
}

// EDIT FORM
pub async fn edit_form(
    Path(id): Path<i32>,
    Extension(state): Extension<Arc<AppState>>,
) -> Result<Html<String>, String> {
    info!("Editing category ID: {}", id);
    
    // Define a serializable struct for the query result
    #[derive(serde::Serialize)]
    struct CategoryRecord {
        id: i32,
        name: String,
    }
    
    let category = sqlx::query_as!(
        CategoryRecord,
        "SELECT id, name FROM categories WHERE id = $1",
        id
    )
    .fetch_one(&state.db)
    .await
    .map_err(|e| {
        warn!("Category {} not found: {}", id, e);
        e.to_string()
    })?;

    let mut ctx = tera::Context::new();
    ctx.insert("category", &category);
    state.templates.render("categories/edit.html", &ctx)
        .map_err(|e| e.to_string())
        .map(Html)
}

// UPDATE
pub async fn update(
    Path(id): Path<i32>,
    Extension(state): Extension<Arc<AppState>>,
    Form(form): Form<CategoryForm>,
) -> Result<Redirect, String> {
    info!("Updating category ID: {}", id);
    
    sqlx::query!(
        "UPDATE categories SET name = $1 WHERE id = $2",
        form.name,
        id
    )
    .execute(&state.db)
    .await
    .map_err(|e| {
        warn!("Category update failed: {}", e);
        e.to_string()
    })?;

    info!("Category {} updated to '{}'", id, form.name);
    Ok(Redirect::to("/categories"))
}

// DELETE
pub async fn delete(
    Path(id): Path<i32>,
    Extension(state): Extension<Arc<AppState>>,
) -> Result<Redirect, String> {
    info!("Deleting category ID: {}", id);
    
    // Check if category is in use
    let equipment_count: i64 = sqlx::query_scalar!(
        "SELECT COUNT(*) FROM equipment WHERE category_id = $1",
        id
    )
    .fetch_one(&state.db)
    .await
    .map_err(|e| {
        warn!("Category deletion check failed: {}", e);
        e.to_string()
    })?
    .unwrap_or(0);
    
    if equipment_count > 0 {
        warn!("Cannot delete category {} with {} equipment items", id, equipment_count);
        return Err(format!("Category is in use by {} equipment items", equipment_count));
    }
    
    sqlx::query!(
        "DELETE FROM categories WHERE id = $1",
        id
    )
    .execute(&state.db)
    .await
    .map_err(|e| {
        warn!("Category deletion failed: {}", e);
        e.to_string()
    })?;

    info!("Category {} deleted", id);
    Ok(Redirect::to("/categories"))
}

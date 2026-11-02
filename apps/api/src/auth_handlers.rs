use axum::{Json, extract::{State, Query}, Extension};
use serde::{Deserialize, Serialize};
use crate::state::AppState;
use application::auth::{LoginCommand, RegisterCommand, Claims};
use policy::Role;
use uuid::Uuid;
use axum::http::StatusCode;
use std::str::FromStr;

#[derive(Deserialize)]
pub struct LoginRequest {
    pub email: String,
    pub password: String,
}

#[derive(Serialize)]
pub struct LoginResponse {
    pub token: String,
    pub user_id: Option<String>,
    pub email: Option<String>,
    pub role: Option<String>,
}

#[derive(Deserialize)]
pub struct RegisterRequest {
    pub email: String,
    pub password: String,
    pub full_name: Option<String>,
    pub role: Option<String>,
    pub organization_id: Option<Uuid>,
}

#[derive(Serialize)]
pub struct RegisterResponse {
    pub user_id: Uuid,
    pub email: String,
    pub role: String,
    pub token: String,
}

#[derive(Deserialize)]
pub struct AuditQuery {
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

pub async fn register_handler(
    State(state): State<AppState>,
    Json(payload): Json<RegisterRequest>,
) -> Result<(StatusCode, Json<RegisterResponse>), StatusCode> {
    let role = payload.role.and_then(|r| Role::from_str(&r).ok());
    let command = RegisterCommand {
        email: payload.email,
        raw_password: payload.password,
        full_name: payload.full_name.unwrap_or_default(),
        role,
        organization_id: payload.organization_id,
        correlation_id: Uuid::new_v4(),
    };

    let (user, token) = state.auth_service.register(command).await
        .map_err(|_| StatusCode::BAD_REQUEST)?;

    Ok((
        StatusCode::CREATED,
        Json(RegisterResponse {
            user_id: user.id().into(),
            email: user.email().as_str().to_string(),
            role: user.role().to_string(),
            token,
        }),
    ))
}

pub async fn login_handler(
    State(state): State<AppState>,
    Json(payload): Json<LoginRequest>,
) -> Result<Json<LoginResponse>, StatusCode> {
    let command = LoginCommand {
        email: payload.email,
        raw_password: payload.password,
        correlation_id: Uuid::new_v4(),
    };

    let token = state.auth_service.login(command).await
        .map_err(|_| StatusCode::UNAUTHORIZED)?;

    let claims = state.auth_service.verify_token(&token)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(LoginResponse {
        token,
        user_id: Some(claims.sub),
        email: Some(claims.email),
        role: Some(claims.role),
    }))
}

pub async fn refresh_handler(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
) -> Result<Json<LoginResponse>, StatusCode> {
    let fresh_token = state.auth_service.refresh_token(&claims.sub).await
        .map_err(|_| StatusCode::UNAUTHORIZED)?;

    let new_claims = state.auth_service.verify_token(&fresh_token)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(LoginResponse {
        token: fresh_token,
        user_id: Some(new_claims.sub),
        email: Some(new_claims.email),
        role: Some(new_claims.role),
    }))
}

pub async fn me_handler(Extension(claims): Extension<Claims>) -> Json<Claims> {
    Json(claims)
}

pub async fn health_handler() -> Json<&'static str> {
    Json("Auth system is healthy")
}

pub async fn get_audit_ledger_handler(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Query(q): Query<AuditQuery>,
) -> Result<Json<Vec<audit::AuditEntry>>, StatusCode> {
    let role = Role::from_str(&claims.role).unwrap_or(Role::Viewer);
    if role == Role::Viewer {
        return Err(StatusCode::FORBIDDEN);
    }

    let limit = q.limit.unwrap_or(50).min(100);
    let offset = q.offset.unwrap_or(0);
    let entries = state.audit_service.get_ledger(limit, offset).await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(entries))
}

pub async fn verify_audit_integrity_handler(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let role = Role::from_str(&claims.role).unwrap_or(Role::Viewer);
    if role != Role::Owner && role != Role::Administrator {
        return Err(StatusCode::FORBIDDEN);
    }

    let valid = state.audit_service.verify_integrity().await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(serde_json::json!({
        "valid": valid,
        "status": "Cryptographic SHA-256 hash chain verified"
    })))
}

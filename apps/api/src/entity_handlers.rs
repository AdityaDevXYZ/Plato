use axum::{Json, extract::{State, Path}, Extension, http::StatusCode};
use serde::{Deserialize, Serialize};
use crate::state::AppState;
use application::auth::Claims;
use uuid::Uuid;
use types::{OrganizationId, ProjectId, MissionId, UserId};
use application::organization::CreateOrganizationCommand;
use application::project::CreateProjectCommand;
use application::mission::CreateMissionCommand;

fn get_user_id(claims: &Claims) -> UserId {
    UserId::from(Uuid::parse_str(&claims.sub).unwrap_or_else(|_| Uuid::new_v4()))
}

// ==========================================
// Organization DTOs & Handlers
// ==========================================

#[derive(Deserialize, Debug)]
pub struct CreateOrgReq {
    pub name: String,
}

#[derive(Serialize, Debug)]
pub struct OrgRes {
    pub id: Uuid,
    pub name: String,
    pub owner_id: Uuid,
    pub status: String,
}

#[tracing::instrument(skip(state, claims))]
pub async fn create_organization(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Json(payload): Json<CreateOrgReq>,
) -> Result<(StatusCode, Json<OrgRes>), StatusCode> {
    let owner_id = get_user_id(&claims);
    let command = CreateOrganizationCommand {
        name: payload.name,
        owner_id,
    };

    let org = state.org_service.create_organization(command).await
        .map_err(|_| StatusCode::BAD_REQUEST)?;

    Ok((StatusCode::CREATED, Json(OrgRes {
        id: org.id().into(),
        name: org.name().as_str().to_string(),
        owner_id: org.owner_id().into(),
        status: format!("{:?}", org.status()),
    })))
}

#[tracing::instrument(skip(state))]
pub async fn get_organization(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<Json<OrgRes>, StatusCode> {
    let org = state.org_service.get_organization(OrganizationId::from(id)).await
        .map_err(|_| StatusCode::NOT_FOUND)?;

    Ok(Json(OrgRes {
        id: org.id().into(),
        name: org.name().as_str().to_string(),
        owner_id: org.owner_id().into(),
        status: format!("{:?}", org.status()),
    }))
}

// ==========================================
// Project DTOs & Handlers
// ==========================================

#[derive(Deserialize, Debug)]
pub struct CreateProjectReq {
    pub name: String,
    pub org_id: Uuid,
}

#[derive(Serialize, Debug)]
pub struct ProjectRes {
    pub id: Uuid,
    pub org_id: Uuid,
    pub name: String,
    pub status: String,
}

#[tracing::instrument(skip(state))]
pub async fn create_project(
    State(state): State<AppState>,
    Json(payload): Json<CreateProjectReq>,
) -> Result<(StatusCode, Json<ProjectRes>), StatusCode> {
    let command = CreateProjectCommand {
        name: payload.name,
        org_id: OrganizationId::from(payload.org_id),
    };

    let proj = state.project_service.create_project(command).await
        .map_err(|_| StatusCode::BAD_REQUEST)?;

    Ok((StatusCode::CREATED, Json(ProjectRes {
        id: proj.id().into(),
        org_id: proj.org_id().into(),
        name: proj.name().as_str().to_string(),
        status: format!("{:?}", proj.status()),
    })))
}

#[tracing::instrument(skip(state))]
pub async fn get_project(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<Json<ProjectRes>, StatusCode> {
    let proj = state.project_service.get_project(ProjectId::from(id)).await
        .map_err(|_| StatusCode::NOT_FOUND)?;

    Ok(Json(ProjectRes {
        id: proj.id().into(),
        org_id: proj.org_id().into(),
        name: proj.name().as_str().to_string(),
        status: format!("{:?}", proj.status()),
    }))
}

// ==========================================
// Mission DTOs & Handlers
// ==========================================

#[derive(Deserialize, Debug)]
pub struct CreateMissionReq {
    pub name: String,
    pub project_id: Uuid,
}

#[derive(Serialize, Debug)]
pub struct MissionRes {
    pub id: Uuid,
    pub project_id: Uuid,
    pub name: String,
    pub status: String,
}

#[tracing::instrument(skip(state))]
pub async fn create_mission(
    State(state): State<AppState>,
    Json(payload): Json<CreateMissionReq>,
) -> Result<(StatusCode, Json<MissionRes>), StatusCode> {
    let command = CreateMissionCommand {
        name: payload.name,
        project_id: ProjectId::from(payload.project_id),
    };

    let mission = state.mission_service.create_mission(command).await
        .map_err(|_| StatusCode::BAD_REQUEST)?;

    Ok((StatusCode::CREATED, Json(MissionRes {
        id: mission.id().into(),
        project_id: mission.project_id().into(),
        name: mission.name().as_str().to_string(),
        status: format!("{:?}", mission.status()),
    })))
}

#[tracing::instrument(skip(state))]
pub async fn get_mission(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<Json<MissionRes>, StatusCode> {
    let mission = state.mission_service.get_mission(MissionId::from(id)).await
        .map_err(|_| StatusCode::NOT_FOUND)?;

    Ok(Json(MissionRes {
        id: mission.id().into(),
        project_id: mission.project_id().into(),
        name: mission.name().as_str().to_string(),
        status: format!("{:?}", mission.status()),
    }))
}

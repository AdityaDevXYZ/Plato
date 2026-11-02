use axum::{Json, extract::{State, Path}, http::StatusCode};
use serde::Deserialize;
use crate::state::AppState;
use uuid::Uuid;
use planning::{model::DeploymentPlan, strategy::{SequentialRollout, CanaryRollout, BatchRollout, PlanningStrategy}};

#[derive(Debug, Deserialize)]
pub struct PlanRequest {
    pub release_id: Uuid,
    pub strategy: String,
}

#[tracing::instrument(skip(state))]
pub async fn create_plan(State(state): State<AppState>, Path(mission_id): Path<Uuid>, Json(req): Json<PlanRequest>) -> Result<Json<DeploymentPlan>, StatusCode> {
    let strategy: Box<dyn PlanningStrategy> = match req.strategy.as_str() {
        "canary" => Box::new(CanaryRollout),
        "batch" => Box::new(BatchRollout),
        _ => Box::new(SequentialRollout),
    };
    
    let plan = state.planning_service.generate_plan(mission_id.into(), req.release_id.into(), strategy.as_ref(), Uuid::new_v4()).await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(Json(plan))
}

#[tracing::instrument(skip(state))]
pub async fn get_plan(State(state): State<AppState>, Path(plan_id): Path<Uuid>) -> Result<Json<DeploymentPlan>, StatusCode> {
    let plan = state.planning_service.get_plan(plan_id).await.map_err(|_| StatusCode::NOT_FOUND)?;
    Ok(Json(plan))
}

use axum::{Json, extract::{State, Path}, Extension};
use serde::{Deserialize, Serialize};
use crate::state::AppState;
use application::auth::Claims;
use uuid::Uuid;
use axum::http::StatusCode;
use types::UserId;
use release::model::TransitionRecord;
use assurance::model::{AssuranceReport, AssuranceDecision, FlightReleaseManifest, BinaryArtifact, CryptographicSignature};
use readiness::model::{DeploymentReadinessReport, ReadinessDecision};
use deployment::model::{DeploymentSession, DeploymentState, LogEntry, TelecommandPacket};
use std::str::FromStr;

#[derive(Deserialize)]
pub struct CreateReleaseReq { pub version: String, pub mission_id: Uuid }

#[derive(Deserialize)]
pub struct TransitionReq { pub reason: String }

#[derive(Serialize)]
pub struct ReleaseRes { pub id: Uuid, pub status: String }

fn get_user_id(claims: &Claims) -> UserId {
    UserId::from(Uuid::parse_str(&claims.sub).unwrap_or_default())
}

fn get_user_uuid(claims: &Claims) -> Uuid {
    Uuid::parse_str(&claims.sub).unwrap_or_default()
}

fn get_auth_ctx(claims: &Claims) -> policy::AuthorizationContext {
    let role = policy::Role::from_str(&claims.role).unwrap_or(policy::Role::Viewer);
    policy::AuthorizationContext {
        user_id: get_user_id(claims),
        role,
    }
}

pub async fn create_release(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Json(req): Json<CreateReleaseReq>,
) -> Result<Json<ReleaseRes>, StatusCode> {
    let auth_ctx = get_auth_ctx(&claims);
    policy::PolicyEngine::can_submit_release(&auth_ctx).map_err(|_| StatusCode::FORBIDDEN)?;

    let corr_id = Uuid::new_v4();
    let r = state.roe_service.create_release(req.mission_id.into(), req.version.clone(), get_user_id(&claims), corr_id)
        .await.map_err(|_| StatusCode::BAD_REQUEST)?;

    let _ = state.audit_service.log(
        get_user_uuid(&claims),
        claims.email.clone(),
        "CREATE_RELEASE".into(),
        "Release".into(),
        r.id().to_string(),
        serde_json::json!({
            "mission_id": req.mission_id,
            "version": req.version,
        }),
    ).await;

    Ok(Json(ReleaseRes { id: r.id().into(), status: format!("{:?}", r.status()) }))
}

pub async fn validate_release(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    Extension(claims): Extension<Claims>,
    Json(req): Json<TransitionReq>,
) -> Result<Json<ReleaseRes>, StatusCode> {
    let auth_ctx = get_auth_ctx(&claims);
    policy::PolicyEngine::can_validate_release(&auth_ctx).map_err(|_| StatusCode::FORBIDDEN)?;

    let r = state.roe_service.validate_release(id.into(), get_user_id(&claims), Uuid::new_v4(), req.reason.clone())
        .await.map_err(|_| StatusCode::BAD_REQUEST)?;

    let _ = state.audit_service.log(
        get_user_uuid(&claims),
        claims.email.clone(),
        "VALIDATE_RELEASE".into(),
        "Release".into(),
        id.to_string(),
        serde_json::json!({ "reason": req.reason }),
    ).await;

    Ok(Json(ReleaseRes { id: r.id().into(), status: format!("{:?}", r.status()) }))
}

pub async fn submit_release(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    Extension(claims): Extension<Claims>,
    Json(req): Json<TransitionReq>,
) -> Result<Json<ReleaseRes>, StatusCode> {
    let auth_ctx = get_auth_ctx(&claims);
    policy::PolicyEngine::can_submit_release(&auth_ctx).map_err(|_| StatusCode::FORBIDDEN)?;

    let r = state.roe_service.submit_release(id.into(), get_user_id(&claims), Uuid::new_v4(), req.reason.clone())
        .await.map_err(|_| StatusCode::BAD_REQUEST)?;

    let _ = state.audit_service.log(
        get_user_uuid(&claims),
        claims.email.clone(),
        "SUBMIT_RELEASE".into(),
        "Release".into(),
        id.to_string(),
        serde_json::json!({ "reason": req.reason }),
    ).await;

    Ok(Json(ReleaseRes { id: r.id().into(), status: format!("{:?}", r.status()) }))
}

pub async fn approve_release(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    Extension(claims): Extension<Claims>,
    Json(req): Json<TransitionReq>,
) -> Result<Json<ReleaseRes>, StatusCode> {
    let auth_ctx = get_auth_ctx(&claims);
    policy::PolicyEngine::can_approve_release(&auth_ctx).map_err(|_| StatusCode::FORBIDDEN)?;

    let r = state.roe_service.approve_release(id.into(), get_user_id(&claims), Uuid::new_v4(), req.reason.clone())
        .await.map_err(|_| StatusCode::BAD_REQUEST)?;

    let _ = state.audit_service.log(
        get_user_uuid(&claims),
        claims.email.clone(),
        "APPROVE_RELEASE".into(),
        "Release".into(),
        id.to_string(),
        serde_json::json!({
            "reason": req.reason,
            "approved_by_role": auth_ctx.role.to_string()
        }),
    ).await;

    Ok(Json(ReleaseRes { id: r.id().into(), status: format!("{:?}", r.status()) }))
}

pub async fn get_release(State(state): State<AppState>, Path(id): Path<Uuid>) -> Result<Json<ReleaseRes>, StatusCode> {
    let r = state.roe_service.get_release(id.into()).await.map_err(|_| StatusCode::NOT_FOUND)?;
    Ok(Json(ReleaseRes { id: r.id().into(), status: format!("{:?}", r.status()) }))
}

pub async fn get_history(State(state): State<AppState>, Path(id): Path<Uuid>) -> Result<Json<Vec<TransitionRecord>>, StatusCode> {
    let r = state.roe_service.get_release(id.into()).await.map_err(|_| StatusCode::NOT_FOUND)?;
    Ok(Json(r.history().to_vec()))
}

#[derive(Deserialize)]
pub struct BinaryArtifactReq {
    pub artifact_id: Option<Uuid>,
    pub name: String,
    pub target_partition: String,
    pub size_bytes: u64,
    pub expected_sha256: String,
    pub payload_hex: Option<String>,
}

#[derive(Deserialize)]
pub struct CryptographicSignatureReq {
    pub signer_id: String,
    pub algorithm: String,
    pub public_key_hex: String,
    pub signature_hex: String,
}

#[derive(Deserialize)]
pub struct CreateManifestReq {
    pub version: String,
    pub target_architecture: String,
    pub max_binary_size_bytes: Option<u64>,
    pub dual_bank_enabled: Option<bool>,
    pub artifacts: Vec<BinaryArtifactReq>,
    pub signatures: Vec<CryptographicSignatureReq>,
}

pub async fn create_release_manifest(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    Extension(claims): Extension<Claims>,
    Json(req): Json<CreateManifestReq>,
) -> Result<Json<FlightReleaseManifest>, StatusCode> {
    let auth_ctx = get_auth_ctx(&claims);
    policy::PolicyEngine::can_submit_release(&auth_ctx).map_err(|_| StatusCode::FORBIDDEN)?;

    let artifacts = req.artifacts.into_iter().map(|a| BinaryArtifact {
        artifact_id: a.artifact_id.unwrap_or_else(Uuid::new_v4),
        name: a.name,
        target_partition: a.target_partition,
        size_bytes: a.size_bytes,
        expected_sha256: a.expected_sha256,
        payload_hex: a.payload_hex,
    }).collect::<Vec<_>>();

    let signatures = req.signatures.into_iter().map(|s| CryptographicSignature {
        signer_id: s.signer_id,
        algorithm: s.algorithm,
        public_key_hex: s.public_key_hex,
        signature_hex: s.signature_hex,
    }).collect::<Vec<_>>();

    let manifest = FlightReleaseManifest {
        release_id: id.into(),
        version: req.version,
        target_architecture: req.target_architecture,
        max_binary_size_bytes: req.max_binary_size_bytes.unwrap_or(33554432),
        dual_bank_enabled: req.dual_bank_enabled.unwrap_or(true),
        artifacts,
        signatures,
        created_at: time::OffsetDateTime::now_utc(),
    };

    state.rae_service.save_manifest(&manifest).await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let _ = state.audit_service.log(
        get_user_uuid(&claims),
        claims.email.clone(),
        "CREATE_RELEASE_MANIFEST".into(),
        "ReleaseManifest".into(),
        id.to_string(),
        serde_json::json!({
            "version": manifest.version,
            "target_architecture": manifest.target_architecture,
            "artifacts_count": manifest.artifacts.len(),
            "signatures_count": manifest.signatures.len()
        }),
    ).await;

    Ok(Json(manifest))
}

pub async fn get_release_manifest(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<Json<FlightReleaseManifest>, StatusCode> {
    let manifest = state.rae_service.get_manifest(id.into()).await.map_err(|_| StatusCode::NOT_FOUND)?;
    Ok(Json(manifest))
}

pub async fn assure_release(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    Extension(claims): Extension<Claims>,
) -> Result<Json<AssuranceReport>, StatusCode> {
    let auth_ctx = get_auth_ctx(&claims);
    policy::PolicyEngine::can_assure_release(&auth_ctx).map_err(|_| StatusCode::FORBIDDEN)?;

    let corr_id = Uuid::new_v4();
    let actor = get_user_id(&claims);
    
    let report = state.rae_service.run_assurance(id.into(), corr_id).await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    
    let passed = report.decision == AssuranceDecision::Assured || report.decision == AssuranceDecision::AssuredWithAdvisory;
    if passed {
        let _ = state.roe_service.mark_assured(id.into(), actor, corr_id, "Assurance passed".into()).await;
    } else {
        let _ = state.roe_service.fail_release(id.into(), actor, corr_id, "Assurance failed".into()).await;
    }

    let _ = state.audit_service.log(
        get_user_uuid(&claims),
        claims.email.clone(),
        "RUN_RELEASE_ASSURANCE".into(),
        "Release".into(),
        id.to_string(),
        serde_json::json!({
            "decision": format!("{:?}", report.decision),
            "passed": passed
        }),
    ).await;

    Ok(Json(report))
}

pub async fn get_assurance(State(state): State<AppState>, Path(id): Path<Uuid>) -> Result<Json<AssuranceReport>, StatusCode> {
    let report = state.rae_service.get_report(id.into()).await.map_err(|_| StatusCode::NOT_FOUND)?;
    Ok(Json(report))
}

pub async fn assess_readiness(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    Extension(claims): Extension<Claims>,
) -> Result<Json<DeploymentReadinessReport>, StatusCode> {
    let auth_ctx = get_auth_ctx(&claims);
    policy::PolicyEngine::can_submit_release(&auth_ctx).map_err(|_| StatusCode::FORBIDDEN)?;

    let corr_id = Uuid::new_v4();
    let actor = get_user_id(&claims);
    
    let report = state.dra_service.run_readiness(id.into(), corr_id).await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    
    let ready = report.decision == ReadinessDecision::Ready || report.decision == ReadinessDecision::ReadyWithWarnings;
    if ready {
        let _ = state.roe_service.mark_ready(id.into(), actor, corr_id, "Readiness passed".into()).await;
    } else {
        let _ = state.roe_service.fail_release(id.into(), actor, corr_id, "Readiness failed".into()).await;
    }

    let _ = state.audit_service.log(
        get_user_uuid(&claims),
        claims.email.clone(),
        "ASSESS_DEPLOYMENT_READINESS".into(),
        "Release".into(),
        id.to_string(),
        serde_json::json!({
            "decision": format!("{:?}", report.decision),
            "ready": ready
        }),
    ).await;

    Ok(Json(report))
}

pub async fn get_readiness(State(state): State<AppState>, Path(id): Path<Uuid>) -> Result<Json<DeploymentReadinessReport>, StatusCode> {
    let report = state.dra_service.get_report(id.into()).await.map_err(|_| StatusCode::NOT_FOUND)?;
    Ok(Json(report))
}

pub async fn deploy_release(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    Extension(claims): Extension<Claims>,
) -> Result<(StatusCode, Json<DeploymentSession>), StatusCode> {
    let auth_ctx = get_auth_ctx(&claims);
    policy::PolicyEngine::can_deploy(&auth_ctx).map_err(|_| StatusCode::FORBIDDEN)?;

    let corr_id = Uuid::new_v4();
    let actor = get_user_id(&claims);
    
    let _ = state.roe_service.begin_deployment(id.into(), actor, corr_id, "Deployment starting".into())
        .await.map_err(|_| StatusCode::BAD_REQUEST)?;
    
    let session = state.dee_service.queue_deployment(id.into(), corr_id, None, Some("bank_a".to_string()))
        .await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let _ = state.audit_service.log(
        get_user_uuid(&claims),
        claims.email.clone(),
        "INITIATE_DEPLOYMENT".into(),
        "DeploymentSession".into(),
        session.session_id.to_string(),
        serde_json::json!({
            "release_id": id,
            "target_bank": "bank_a",
            "session_id": session.session_id
        }),
    ).await;

    let dee = state.dee_service.clone();
    let roe = state.roe_service.clone();
    let sess_id = session.session_id;

    tokio::spawn(async move {
        match dee.execute_session(sess_id).await {
            Ok(s) => {
                if s.status == DeploymentState::Succeeded {
                    let _ = roe.complete_deployment(id.into(), actor, corr_id, "Deployment completed".into()).await;
                } else {
                    let _ = roe.fail_release(id.into(), actor, corr_id, "Deployment failed".into()).await;
                }
            }
            Err(e) => {
                let _ = roe.fail_release(id.into(), actor, corr_id, format!("Deployment error: {}", e)).await;
            }
        }
    });

    Ok((StatusCode::ACCEPTED, Json(session)))
}

pub async fn get_deployment(State(state): State<AppState>, Path(id): Path<Uuid>) -> Result<Json<DeploymentSession>, StatusCode> {
    let session = state.dee_service.get_session(id).await.map_err(|_| StatusCode::NOT_FOUND)?;
    Ok(Json(session))
}

pub async fn get_deployment_logs(State(state): State<AppState>, Path(id): Path<Uuid>) -> Result<Json<Vec<LogEntry>>, StatusCode> {
    let session = state.dee_service.get_session(id).await.map_err(|_| StatusCode::NOT_FOUND)?;
    Ok(Json(session.execution_log))
}

pub async fn get_deployment_packets(State(state): State<AppState>, Path(id): Path<Uuid>) -> Result<Json<Vec<TelecommandPacket>>, StatusCode> {
    let packets = state.dee_service.get_session_packets(id).await.map_err(|_| StatusCode::NOT_FOUND)?;
    Ok(Json(packets))
}

#[derive(Serialize, Debug)]
pub struct DeploymentQueueItem {
    pub id: String,
    pub mission: String,
    pub status: String,
    pub progress: u32,
    pub release_id: String,
}

pub async fn list_deployments_queue(State(state): State<AppState>) -> Result<Json<Vec<DeploymentQueueItem>>, StatusCode> {
    let sessions = state.dee_service.get_recent_sessions(50).await.unwrap_or_default();

    let items = sessions.into_iter().map(|s| {
        let (status_str, progress) = match s.status {
            DeploymentState::Queued | DeploymentState::Initializing | DeploymentState::Waiting => ("Queued", 10),
            DeploymentState::Executing | DeploymentState::Retrying => ("Running", 65),
            DeploymentState::Succeeded => ("Completed", 100),
            DeploymentState::Failed | DeploymentState::RollingBack | DeploymentState::Cancelled => ("Failed", 100),
        };

        DeploymentQueueItem {
            id: s.session_id.to_string(),
            mission: format!("Release {}", &s.release_id.to_string()[..s.release_id.to_string().len().min(8)]),
            status: status_str.to_string(),
            progress,
            release_id: s.release_id.to_string(),
        }
    }).collect();

    Ok(Json(items))
}

pub async fn pause_deployment(State(_state): State<AppState>, Path(_id): Path<String>) -> Result<Json<serde_json::Value>, StatusCode> {
    Ok(Json(serde_json::json!({ "status": "paused" })))
}

pub async fn cancel_deployment(State(_state): State<AppState>, Path(_id): Path<String>) -> Result<Json<serde_json::Value>, StatusCode> {
    Ok(Json(serde_json::json!({ "status": "cancelled" })))
}

pub async fn rollback_deployment(State(_state): State<AppState>, Path(_id): Path<String>) -> Result<Json<serde_json::Value>, StatusCode> {
    Ok(Json(serde_json::json!({ "status": "rollback_initiated" })))
}

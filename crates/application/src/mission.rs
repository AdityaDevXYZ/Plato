use crate::error::ApplicationError;
use mission::{
    model::{Mission, MissionName},
    repository::MissionRepository,
};
use types::ProjectId;
use std::sync::Arc;

pub struct CreateMissionCommand {
    pub name: String,
    pub project_id: ProjectId,
}

/// MissionService coordinates mission use cases.
///
/// RESPONSIBILITY: Orchestrating the lifecycle of a Mission.
/// WHAT BELONGS HERE: Cross-entity coordination and use case execution.
/// WHAT DOES NOT BELONG HERE: State transition rules (e.g., verifying if a mission can complete).
pub struct MissionService {
    mission_repo: Arc<dyn MissionRepository>,
}

impl MissionService {
    pub fn new(mission_repo: Arc<dyn MissionRepository>) -> Self {
        Self { mission_repo }
    }

    pub async fn create_mission(
        &self,
        command: CreateMissionCommand,
    ) -> Result<Mission, ApplicationError> {
        let name = MissionName::new(command.name)
            .map_err(|e| ApplicationError::Domain(e.to_string()))?;

        let mission = Mission::new(name, command.project_id);

        self.mission_repo
            .save(&mission)
            .await.map_err(|e| ApplicationError::Repository(e.to_string()))?;

        Ok(mission)
    }

    pub async fn get_mission(
        &self,
        id: types::MissionId,
    ) -> Result<Mission, ApplicationError> {
        self.mission_repo
            .find_by_id(id)
            .await
            .map_err(|e| ApplicationError::Repository(e.to_string()))?
            .ok_or_else(|| ApplicationError::NotFound("Mission not found".into()))
    }
}

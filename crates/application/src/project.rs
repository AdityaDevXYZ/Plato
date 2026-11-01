use crate::error::ApplicationError;
use project::{
    model::{Project, ProjectName},
    repository::ProjectRepository,
};
use types::OrganizationId;
use std::sync::Arc;

pub struct CreateProjectCommand {
    pub name: String,
    pub org_id: OrganizationId,
}

/// ProjectService coordinates project use cases.
///
/// RESPONSIBILITY: Orchestrating the creation and management of Projects.
/// WHAT BELONGS HERE: Coordination of repositories (e.g., fetching an org before saving a project).
/// WHAT DOES NOT BELONG HERE: Domain rules or database specifics.
pub struct ProjectService {
    project_repo: Arc<dyn ProjectRepository>,
}

impl ProjectService {
    pub fn new(project_repo: Arc<dyn ProjectRepository>) -> Self {
        Self { project_repo }
    }

    pub async fn create_project(
        &self,
        command: CreateProjectCommand,
    ) -> Result<Project, ApplicationError> {
        let name = ProjectName::new(command.name)
            .map_err(|e| ApplicationError::Domain(e.to_string()))?;

        let project = Project::new(name, command.org_id);

        self.project_repo
            .save(&project)
            .await.map_err(|e| ApplicationError::Repository(e.to_string()))?;

        Ok(project)
    }

    pub async fn get_project(
        &self,
        id: types::ProjectId,
    ) -> Result<Project, ApplicationError> {
        self.project_repo
            .find_by_id(id)
            .await
            .map_err(|e| ApplicationError::Repository(e.to_string()))?
            .ok_or_else(|| ApplicationError::NotFound("Project not found".into()))
    }
}

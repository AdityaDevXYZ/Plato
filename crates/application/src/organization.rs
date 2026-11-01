use crate::error::ApplicationError;
use organization::{
    model::{Organization, OrganizationName},
    repository::OrganizationRepository,
};
use types::UserId;
use std::sync::Arc;

pub struct CreateOrganizationCommand {
    pub name: String,
    pub owner_id: UserId,
}

/// OrganizationService coordinates organizational use cases.
/// 
/// RESPONSIBILITY:
/// - Orchestrating the creation and management of Organizations.
/// - Injecting domain repositories.
/// 
/// WHAT BELONGS HERE:
/// - Use case execution logic (e.g., verifying a user exists before creating an org).
/// - Transaction boundaries (conceptually).
/// 
/// WHAT DOES NOT BELONG HERE:
/// - Core business rules (e.g., organization name constraints, state transitions).
/// - Infrastructure details (e.g., SQL queries, HTTP request parsing).
pub struct OrganizationService {
    org_repo: Arc<dyn OrganizationRepository>,
}

impl OrganizationService {
    pub fn new(org_repo: Arc<dyn OrganizationRepository>) -> Self {
        Self { org_repo }
    }

    pub async fn create_organization(
        &self,
        command: CreateOrganizationCommand,
    ) -> Result<Organization, ApplicationError> {
        // 1. Basic validation & Value Object creation
        let name = OrganizationName::new(command.name)
            .map_err(|e| ApplicationError::Domain(e.to_string()))?;

        // 2. Domain behavior (Entity creation)
        let org = Organization::new(name, command.owner_id);

        // 3. Persistence (via Repository Trait)
        self.org_repo
            .save(&org)
            .await.map_err(|e| ApplicationError::Repository(e.to_string()))?;

        // 4. Return result
        Ok(org)
    }

    pub async fn get_organization(
        &self,
        id: types::OrganizationId,
    ) -> Result<Organization, ApplicationError> {
        self.org_repo
            .find_by_id(id)
            .await
            .map_err(|e| ApplicationError::Repository(e.to_string()))?
            .ok_or_else(|| ApplicationError::NotFound("Organization not found".into()))
    }
}

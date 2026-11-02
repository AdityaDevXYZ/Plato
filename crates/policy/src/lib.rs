use types::{UserId, OrganizationId, ProjectId};
use thiserror::Error;
use serde::{Serialize, Deserialize};
use std::fmt;
use std::str::FromStr;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Role {
    Owner,
    Administrator,
    Operator,
    Viewer,
}

impl fmt::Display for Role {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Role::Owner => write!(f, "Owner"),
            Role::Administrator => write!(f, "Administrator"),
            Role::Operator => write!(f, "Operator"),
            Role::Viewer => write!(f, "Viewer"),
        }
    }
}

impl FromStr for Role {
    type Err = std::convert::Infallible;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.trim().to_lowercase().as_str() {
            "owner" => Ok(Role::Owner),
            "admin" | "administrator" => Ok(Role::Administrator),
            "operator" => Ok(Role::Operator),
            _ => Ok(Role::Viewer),
        }
    }
}

#[derive(Debug, Error)]
pub enum AuthorizationError {
    #[error("Insufficient permissions: required higher privileged role")]
    Forbidden,
}

#[derive(Debug, Clone)]
pub struct AuthorizationContext {
    pub user_id: UserId,
    pub role: Role,
}

pub struct PolicyEngine;

impl PolicyEngine {
    pub fn can_create_project(ctx: &AuthorizationContext, _target_org: OrganizationId) -> Result<(), AuthorizationError> {
        match ctx.role {
            Role::Owner | Role::Administrator => Ok(()),
            _ => Err(AuthorizationError::Forbidden),
        }
    }

    pub fn can_approve_release(ctx: &AuthorizationContext) -> Result<(), AuthorizationError> {
        match ctx.role {
            Role::Owner | Role::Administrator => Ok(()),
            _ => Err(AuthorizationError::Forbidden),
        }
    }

    pub fn can_submit_release(ctx: &AuthorizationContext) -> Result<(), AuthorizationError> {
        match ctx.role {
            Role::Owner | Role::Administrator | Role::Operator => Ok(()),
            _ => Err(AuthorizationError::Forbidden),
        }
    }

    pub fn can_validate_release(ctx: &AuthorizationContext) -> Result<(), AuthorizationError> {
        match ctx.role {
            Role::Owner | Role::Administrator | Role::Operator => Ok(()),
            _ => Err(AuthorizationError::Forbidden),
        }
    }

    pub fn can_assure_release(ctx: &AuthorizationContext) -> Result<(), AuthorizationError> {
        match ctx.role {
            Role::Owner | Role::Administrator | Role::Operator => Ok(()),
            _ => Err(AuthorizationError::Forbidden),
        }
    }

    pub fn can_deploy_release(ctx: &AuthorizationContext, _target_project: ProjectId) -> Result<(), AuthorizationError> {
        match ctx.role {
            Role::Owner | Role::Administrator | Role::Operator => Ok(()),
            _ => Err(AuthorizationError::Forbidden),
        }
    }

    pub fn can_deploy(ctx: &AuthorizationContext) -> Result<(), AuthorizationError> {
        match ctx.role {
            Role::Owner | Role::Administrator | Role::Operator => Ok(()),
            _ => Err(AuthorizationError::Forbidden),
        }
    }

    pub fn can_trigger_rollback(ctx: &AuthorizationContext) -> Result<(), AuthorizationError> {
        match ctx.role {
            Role::Owner | Role::Administrator | Role::Operator => Ok(()),
            _ => Err(AuthorizationError::Forbidden),
        }
    }

    pub fn can_manage_satellites(ctx: &AuthorizationContext) -> Result<(), AuthorizationError> {
        match ctx.role {
            Role::Owner | Role::Administrator | Role::Operator => Ok(()),
            _ => Err(AuthorizationError::Forbidden),
        }
    }

    pub fn can_manage_users(ctx: &AuthorizationContext) -> Result<(), AuthorizationError> {
        match ctx.role {
            Role::Owner | Role::Administrator => Ok(()),
            _ => Err(AuthorizationError::Forbidden),
        }
    }
    
    pub fn can_view_mission(_ctx: &AuthorizationContext) -> Result<(), AuthorizationError> {
        Ok(()) // Viewers and above
    }

    pub fn can_view(_ctx: &AuthorizationContext) -> Result<(), AuthorizationError> {
        Ok(()) // Viewers and above
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_role_parsing_and_display() {
        assert_eq!(Role::from_str("Owner").unwrap(), Role::Owner);
        assert_eq!(Role::from_str("admin").unwrap(), Role::Administrator);
        assert_eq!(Role::from_str("operator").unwrap(), Role::Operator);
        assert_eq!(Role::from_str("unknown").unwrap(), Role::Viewer);
        assert_eq!(Role::Owner.to_string(), "Owner");
    }

    #[test]
    fn test_dual_control_release_approval() {
        let admin_ctx = AuthorizationContext { user_id: UserId::new(), role: Role::Administrator };
        let operator_ctx = AuthorizationContext { user_id: UserId::new(), role: Role::Operator };
        let viewer_ctx = AuthorizationContext { user_id: UserId::new(), role: Role::Viewer };

        // Only Admin/Owner can approve release
        assert!(PolicyEngine::can_approve_release(&admin_ctx).is_ok());
        assert!(PolicyEngine::can_approve_release(&operator_ctx).is_err());
        assert!(PolicyEngine::can_approve_release(&viewer_ctx).is_err());

        // Operators can deploy approved releases
        assert!(PolicyEngine::can_deploy_release(&operator_ctx, ProjectId::new()).is_ok());
        assert!(PolicyEngine::can_deploy_release(&viewer_ctx, ProjectId::new()).is_err());
    }
}

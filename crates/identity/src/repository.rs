use crate::user::User;
use types::{UserId, Email};
use thiserror::Error;
use async_trait::async_trait;

#[derive(Debug, Error)]
pub enum RepositoryError {
    #[error("User not found")]
    NotFound,
    #[error("Database error: {0}")]
    Database(String),
}

#[async_trait]
pub trait UserRepository: Send + Sync {
    async fn save(&self, user: &User) -> Result<(), RepositoryError>;
    async fn find_by_id(&self, id: UserId) -> Result<Option<User>, RepositoryError>;
    async fn find_by_email(&self, email: &Email) -> Result<Option<User>, RepositoryError>;
    async fn find_all(&self) -> Result<Vec<User>, RepositoryError>;
}

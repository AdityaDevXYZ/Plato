use types::{UserId, Email};
use policy::Role;
use uuid::Uuid;
use thiserror::Error;
use argon2::{
    password_hash::{rand_core::OsRng, PasswordHash, PasswordHasher, PasswordVerifier, SaltString},
    Argon2,
};
use time::OffsetDateTime;

#[derive(Debug, Error)]
pub enum UserError {
    #[error("User is already active")]
    AlreadyActive,
    #[error("User is deactivated")]
    Deactivated,
    #[error("Password verification failed")]
    InvalidPassword,
    #[error("Hashing error")]
    HashingError,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UserStatus {
    Active,
    Inactive,
}

#[derive(Debug, Clone)]
pub struct User {
    id: UserId,
    email: Email,
    password_hash: String,
    password_updated_at: OffsetDateTime,
    email_verified: bool,
    status: UserStatus,
    role: Role,
    full_name: String,
    organization_id: Option<Uuid>,
}

impl User {
    pub fn reconstruct(
        id: UserId,
        email: Email,
        password_hash: String,
        password_updated_at: OffsetDateTime,
        email_verified: bool,
        status: UserStatus,
        role: Role,
        full_name: String,
        organization_id: Option<Uuid>,
    ) -> Self {
        Self {
            id,
            email,
            password_hash,
            password_updated_at,
            email_verified,
            status,
            role,
            full_name,
            organization_id,
        }
    }

    pub fn new(
        email: Email,
        raw_password: &str,
        role: Role,
        full_name: String,
        organization_id: Option<Uuid>,
    ) -> Result<Self, UserError> {
        let salt = SaltString::generate(&mut OsRng);
        let argon2 = Argon2::default();
        let password_hash = argon2
            .hash_password(raw_password.as_bytes(), &salt)
            .map_err(|_| UserError::HashingError)?
            .to_string();

        Ok(Self {
            id: UserId::new(),
            email,
            password_hash,
            password_updated_at: OffsetDateTime::now_utc(),
            email_verified: false,
            status: UserStatus::Active,
            role,
            full_name,
            organization_id,
        })
    }

    pub fn new_operator(email: Email, raw_password: &str) -> Result<Self, UserError> {
        Self::new(email, raw_password, Role::Operator, String::new(), None)
    }

    pub fn id(&self) -> UserId {
        self.id
    }

    pub fn email(&self) -> &Email {
        &self.email
    }

    pub fn password_hash(&self) -> &str {
        &self.password_hash
    }

    pub fn password_updated_at(&self) -> OffsetDateTime {
        self.password_updated_at
    }

    pub fn email_verified(&self) -> bool {
        self.email_verified
    }

    pub fn status(&self) -> &UserStatus {
        &self.status
    }

    pub fn role(&self) -> Role {
        self.role
    }

    pub fn full_name(&self) -> &str {
        &self.full_name
    }

    pub fn organization_id(&self) -> Option<Uuid> {
        self.organization_id
    }

    pub fn verify_password(&self, raw_password: &str) -> Result<(), UserError> {
        let parsed_hash = PasswordHash::new(&self.password_hash)
            .map_err(|_| UserError::HashingError)?;

        Argon2::default()
            .verify_password(raw_password.as_bytes(), &parsed_hash)
            .map_err(|_| UserError::InvalidPassword)?;

        Ok(())
    }

    pub fn deactivate(&mut self) -> Result<(), UserError> {
        if self.status == UserStatus::Inactive {
            return Err(UserError::Deactivated);
        }
        self.status = UserStatus::Inactive;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use types::Email;

    #[test]
    fn test_password_hashing_and_verification() {
        let email = Email::new("test@example.com").unwrap();
        let user = User::new_operator(email, "supersecret123").unwrap();

        assert_eq!(user.role(), Role::Operator);
        assert!(user.verify_password("supersecret123").is_ok());
        assert!(user.verify_password("wrongpassword").is_err());
    }
}

use crate::error::ApplicationError;
use audit::AuditLedgerRepository;
use identity::{repository::UserRepository, user::User};
use policy::Role;
use types::{Email, UserId};
use std::sync::Arc;
use eventbus::{EventPublisher, EventMessage, Event};
use jsonwebtoken::{encode, decode, Header, Validation, EncodingKey, DecodingKey};
use serde::{Serialize, Deserialize};
use time::{OffsetDateTime, Duration};
use uuid::Uuid;
use std::any::Any;

#[derive(Debug, Clone)]
pub struct UserLoggedIn {
    pub user_id: String,
}
impl Event for UserLoggedIn {
    fn event_type(&self) -> &'static str {
        "UserLoggedIn"
    }
    fn as_any(&self) -> &dyn Any {
        self
    }
}

#[derive(Debug, Clone)]
pub struct UserRegistered {
    pub user_id: String,
    pub email: String,
    pub role: String,
}
impl Event for UserRegistered {
    fn event_type(&self) -> &'static str {
        "UserRegistered"
    }
    fn as_any(&self) -> &dyn Any {
        self
    }
}

#[derive(Debug, Clone)]
pub struct UserLoggedOut {
    pub user_id: String,
}
impl Event for UserLoggedOut {
    fn event_type(&self) -> &'static str {
        "UserLoggedOut"
    }
    fn as_any(&self) -> &dyn Any {
        self
    }
}

#[derive(Debug, Clone)]
pub struct AuthenticationFailed {
    pub email: String,
    pub reason: String,
}
impl Event for AuthenticationFailed {
    fn event_type(&self) -> &'static str {
        "AuthenticationFailed"
    }
    fn as_any(&self) -> &dyn Any {
        self
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Claims {
    pub sub: String,
    #[serde(default)]
    pub email: String,
    pub role: String,
    pub exp: i64,
}

#[derive(Debug, Clone)]
pub struct LoginCommand {
    pub email: String,
    pub raw_password: String,
    pub correlation_id: Uuid,
}

#[derive(Debug, Clone)]
pub struct RegisterCommand {
    pub email: String,
    pub raw_password: String,
    pub full_name: String,
    pub role: Option<Role>,
    pub organization_id: Option<Uuid>,
    pub correlation_id: Uuid,
}

pub struct AuthenticationService {
    user_repo: Arc<dyn UserRepository>,
    event_publisher: Arc<dyn EventPublisher>,
    jwt_secret: String,
    audit_repo: Option<Arc<dyn AuditLedgerRepository>>,
}

impl AuthenticationService {
    pub fn new(
        user_repo: Arc<dyn UserRepository>,
        event_publisher: Arc<dyn EventPublisher>,
        jwt_secret: String,
    ) -> Self {
        Self {
            user_repo,
            event_publisher,
            jwt_secret,
            audit_repo: None,
        }
    }

    pub fn with_audit(
        user_repo: Arc<dyn UserRepository>,
        event_publisher: Arc<dyn EventPublisher>,
        jwt_secret: String,
        audit_repo: Arc<dyn AuditLedgerRepository>,
    ) -> Self {
        Self {
            user_repo,
            event_publisher,
            jwt_secret,
            audit_repo: Some(audit_repo),
        }
    }

    #[tracing::instrument(skip(self))]
    pub async fn register(&self, command: RegisterCommand) -> Result<(User, String), ApplicationError> {
        let email = Email::new(command.email.clone())
            .map_err(|_| ApplicationError::Validation("Invalid email format".into()))?;

        if command.raw_password.len() < 8 {
            return Err(ApplicationError::Validation(
                "Password must be at least 8 characters long".into(),
            ));
        }

        // Check if user already exists
        if self.user_repo.find_by_email(&email).await
            .map_err(|e| ApplicationError::Repository(e.to_string()))?
            .is_some()
        {
            return Err(ApplicationError::Validation(format!(
                "User with email '{}' already exists",
                command.email
            )));
        }

        let assigned_role = command.role.unwrap_or(Role::Operator);
        let user = User::new(
            email,
            &command.raw_password,
            assigned_role,
            command.full_name,
            command.organization_id,
        )
        .map_err(|e| ApplicationError::Domain(format!("User creation error: {}", e)))?;

        self.user_repo
            .save(&user)
            .await
            .map_err(|e| ApplicationError::Repository(e.to_string()))?;

        let user_id_str = Uuid::from(user.id()).to_string();

        // Audit log registration
        if let Some(audit) = &self.audit_repo {
            let _ = audit.append_entry(
                user.id().into(),
                command.email.clone(),
                "USER_REGISTER".into(),
                "User".into(),
                user_id_str.clone(),
                serde_json::json!({
                    "role": assigned_role.to_string(),
                    "email": command.email,
                }),
            ).await;
        }

        let _ = self.event_publisher.publish(vec![EventMessage::new(
            Box::new(UserRegistered {
                user_id: user_id_str.clone(),
                email: command.email.clone(),
                role: assigned_role.to_string(),
            }),
            command.correlation_id,
            "auth",
        )]);

        // Generate initial JWT
        let expiration = OffsetDateTime::now_utc() + Duration::hours(24);
        let claims = Claims {
            sub: user_id_str,
            email: command.email,
            role: assigned_role.to_string(),
            exp: expiration.unix_timestamp(),
        };

        let token = encode(
            &Header::default(),
            &claims,
            &EncodingKey::from_secret(self.jwt_secret.as_bytes()),
        )
        .map_err(|e| ApplicationError::Domain(format!("JWT Error: {}", e)))?;

        Ok((user, token))
    }

    #[tracing::instrument(skip(self))]
    pub async fn login(&self, command: LoginCommand) -> Result<String, ApplicationError> {
        let email = Email::new(command.email.clone())
            .map_err(|_| ApplicationError::Validation("Invalid email".into()))?;

        let user_opt = self
            .user_repo
            .find_by_email(&email)
            .await
            .map_err(|e| ApplicationError::Repository(e.to_string()))?;

        let user = match user_opt {
            Some(u) => u,
            None => {
                let fail = EventMessage::new(
                    Box::new(AuthenticationFailed {
                        email: command.email,
                        reason: "User not found".into(),
                    }),
                    command.correlation_id,
                    "auth",
                );
                let _ = self.event_publisher.publish(vec![fail]);
                return Err(ApplicationError::Validation("Invalid credentials".into()));
            }
        };

        if user.verify_password(&command.raw_password).is_err() {
            let fail = EventMessage::new(
                Box::new(AuthenticationFailed {
                    email: command.email,
                    reason: "Invalid password".into(),
                }),
                command.correlation_id,
                "auth",
            );
            let _ = self.event_publisher.publish(vec![fail]);
            return Err(ApplicationError::Validation("Invalid credentials".into()));
        }

        let expiration = OffsetDateTime::now_utc() + Duration::hours(24);
        let user_id_str = Uuid::from(user.id()).to_string();
        let role_str = user.role().to_string();

        let claims = Claims {
            sub: user_id_str.clone(),
            email: command.email.clone(),
            role: role_str.clone(),
            exp: expiration.unix_timestamp(),
        };

        let token = encode(
            &Header::default(),
            &claims,
            &EncodingKey::from_secret(self.jwt_secret.as_bytes()),
        )
        .map_err(|e| ApplicationError::Domain(format!("JWT Error: {}", e)))?;

        // Audit log login
        if let Some(audit) = &self.audit_repo {
            let _ = audit.append_entry(
                user.id().into(),
                command.email,
                "USER_LOGIN".into(),
                "User".into(),
                user_id_str.clone(),
                serde_json::json!({ "role": role_str }),
            ).await;
        }

        let success = EventMessage::new(
            Box::new(UserLoggedIn {
                user_id: user_id_str,
            }),
            command.correlation_id,
            "auth",
        );
        let _ = self.event_publisher.publish(vec![success]);

        Ok(token)
    }

    pub fn verify_token(&self, token: &str) -> Result<Claims, ApplicationError> {
        let token_data = decode::<Claims>(
            token,
            &DecodingKey::from_secret(self.jwt_secret.as_bytes()),
            &Validation::default(),
        )
        .map_err(|_| ApplicationError::Validation("Invalid or expired token".into()))?;

        Ok(token_data.claims)
    }

    #[tracing::instrument(skip(self))]
    pub async fn refresh_token(&self, token: &str) -> Result<String, ApplicationError> {
        // Decode without enforcing expiry so a slightly expired token can be refreshed
        let mut validation = Validation::default();
        validation.validate_exp = false;

        let token_data = decode::<Claims>(
            token,
            &DecodingKey::from_secret(self.jwt_secret.as_bytes()),
            &validation,
        )
        .map_err(|_| ApplicationError::Validation("Invalid token format".into()))?;

        let user_uuid = Uuid::parse_str(&token_data.claims.sub)
            .map_err(|_| ApplicationError::Validation("Invalid token subject".into()))?;

        let user_id = UserId::from(user_uuid);
        let user = self
            .user_repo
            .find_by_id(user_id)
            .await
            .map_err(|e| ApplicationError::Repository(e.to_string()))?
            .ok_or_else(|| ApplicationError::NotFound("User not found".into()))?;

        let expiration = OffsetDateTime::now_utc() + Duration::hours(24);
        let fresh_claims = Claims {
            sub: user_uuid.to_string(),
            email: user.email().as_str().to_string(),
            role: user.role().to_string(),
            exp: expiration.unix_timestamp(),
        };

        encode(
            &Header::default(),
            &fresh_claims,
            &EncodingKey::from_secret(self.jwt_secret.as_bytes()),
        )
        .map_err(|e| ApplicationError::Domain(format!("JWT Error: {}", e)))
    }

    pub fn logout(&self, user_id: String, correlation_id: Uuid) -> Result<(), ApplicationError> {
        let event = EventMessage::new(Box::new(UserLoggedOut { user_id }), correlation_id, "auth");
        let _ = self.event_publisher.publish(vec![event]);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_jwt_validation_and_expiration() {
        let secret = "supersecret".to_string();
        let expiration = OffsetDateTime::now_utc() - Duration::hours(1); // expired
        let claims = Claims {
            sub: "user-123".into(),
            email: "user123@plato.space".into(),
            role: "Viewer".into(),
            exp: expiration.unix_timestamp(),
        };
        let token = encode(
            &Header::default(),
            &claims,
            &EncodingKey::from_secret(secret.as_bytes()),
        )
        .unwrap();

        let decode_result = decode::<Claims>(
            &token,
            &DecodingKey::from_secret(secret.as_bytes()),
            &Validation::default(),
        );
        assert!(decode_result.is_err()); // Expired
    }

    #[test]
    fn test_valid_jwt_and_role_extraction() {
        let secret = "plato-production-secret-key-12345".to_string();
        let expiration = OffsetDateTime::now_utc() + Duration::hours(2);
        let claims = Claims {
            sub: Uuid::new_v4().to_string(),
            email: "director@satellite-fleet.com".into(),
            role: "Administrator".into(),
            exp: expiration.unix_timestamp(),
        };

        let token = encode(
            &Header::default(),
            &claims,
            &EncodingKey::from_secret(secret.as_bytes()),
        )
        .unwrap();

        let decoded = decode::<Claims>(
            &token,
            &DecodingKey::from_secret(secret.as_bytes()),
            &Validation::default(),
        )
        .unwrap();

        assert_eq!(decoded.claims.email, "director@satellite-fleet.com");
        assert_eq!(decoded.claims.role, "Administrator");
        assert_eq!(decoded.claims.role.parse::<Role>().unwrap(), Role::Administrator);
    }
}

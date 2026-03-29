use jsonwebtoken::{DecodingKey, EncodingKey, Header, Validation, decode, encode};
use serde::{Deserialize, Serialize};

use crate::domain::error::AppError;

#[derive(Debug, Serialize, Deserialize)]
pub struct Claims {
    pub user_id: i64,
    pub username: String,
    pub exp: usize,
}

#[derive(Clone)]
pub struct JwtService {
    encoding_key: EncodingKey,
    decoding_key: DecodingKey,
}

impl JwtService {
    pub fn new(secret: &str) -> Self {
        Self {
            encoding_key: EncodingKey::from_secret(secret.as_bytes()),
            decoding_key: DecodingKey::from_secret(secret.as_bytes()),
        }
    }

    pub fn generate_token(&self, user_id: i64, username: &str) -> Result<String, AppError> {
        let expiration = chrono::Utc::now()
            .checked_add_signed(chrono::Duration::hours(24))
            .expect("valid timestamp")
            .timestamp() as usize;

        let claims = Claims { user_id, username: username.to_string(), exp: expiration };

        encode(&Header::default(), &claims, &self.encoding_key)
            .map_err(|e| AppError::Internal(e.to_string()))
    }

    pub fn validate_token(&self, token: &str) -> Result<Claims, AppError> {
        decode::<Claims>(token, &self.decoding_key, &Validation::default())
            .map(|data| data.claims)
            .map_err(|_| AppError::Unauthorized)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_jwt_round_trip() {
        let service = JwtService::new("test-secret");
        let token = service.generate_token(42, "testuser").unwrap();
        let claims = service.validate_token(&token).unwrap();
        assert_eq!(claims.user_id, 42);
        assert_eq!(claims.username, "testuser");
    }

    #[test]
    fn test_jwt_invalid_token() {
        let service = JwtService::new("test-secret");
        let result = service.validate_token("invalid.token.here");
        assert!(result.is_err());
    }
}

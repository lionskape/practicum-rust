use crate::{
    data::user_repository::UserRepository,
    domain::{error::AppError, user::User},
    infrastructure::{jwt::JwtService, password},
};

pub struct AuthService<U: UserRepository> {
    user_repo: U,
    jwt: JwtService,
}

pub struct AuthResult {
    pub token: String,
    pub user: User,
}

impl<U: UserRepository> AuthService<U> {
    pub fn new(user_repo: U, jwt: JwtService) -> Self {
        Self { user_repo, jwt }
    }

    pub async fn register(
        &self,
        username: &str,
        email: &str,
        password_raw: &str,
    ) -> Result<AuthResult, AppError> {
        let password_hash = password::hash_password(password_raw)?;
        let user = self.user_repo.create(username, email, &password_hash).await?;
        let token = self.jwt.generate_token(user.id, &user.username)?;
        Ok(AuthResult { token, user })
    }

    pub async fn login(&self, username: &str, password_raw: &str) -> Result<AuthResult, AppError> {
        let user = self.user_repo.find_by_username(username).await.map_err(|e| match e {
            AppError::UserNotFound => AppError::InvalidCredentials,
            other => other,
        })?;

        if !password::verify_password(password_raw, &user.password_hash)? {
            return Err(AppError::InvalidCredentials);
        }

        let token = self.jwt.generate_token(user.id, &user.username)?;
        Ok(AuthResult { token, user })
    }
}

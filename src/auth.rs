use axum::extract::Request;
use axum::http::header;
use axum::response::IntoResponse;
use axum::Json;
use chrono::{Duration, Utc};
use jsonwebtoken::{Algorithm, DecodingKey, EncodingKey, TokenData, Validation};
use secrecy::{ExposeSecret as _, SecretString};

use crate::prelude::*;

pub mod prelude {
    pub use super::{AuthError, Authenticator, RegistrationToken, User, UserCredentials};
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize, new)]
pub struct User {
    pub id: Record<User>,
    pub created_at: Timestamp,
    pub username: String,
}

define_table!("users" : User = id);

define_relation! {
    User > find(username: &str, password: &str) > Only<User>
        where "SELECT * FROM users WHERE allowed_signin = true AND username = $username AND crypto::argon2::compare(password, $password) LIMIT 1"
}

define_relation! {
    User > create(username: &str, password: &str) > Only<User>
        where "CREATE users SET username = $username, password = crypto::argon2::generate($password) RETURN *"
}

define_relation! {
    User > by_username(username: &str) > Option<User>
        where "SELECT * FROM users WHERE username = $username LIMIT 1"
}

impl User {
    pub async fn can_generate_token(
        id: &Record<User>,
        db: &Database,
    ) -> Result<bool, DatabaseQueryError> {
        let query: Option<()> = db
            .sql("SELECT id FROM users WHERE id = $id AND roles CONTAINS 'admin' LIMIT 1")
            .bind(("id", id))
            .fetch_first()
            .await?;

        Ok(query.is_some())
    }
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize, new)]
pub struct RegistrationToken {
    pub id: Record<Self>,
    pub created_at: Timestamp,
    pub created_by: Record<User>,
    pub reason: Option<String>,
}

define_table!("registration_tokens" : RegistrationToken = id);

define_relation! {
    RegistrationToken > issue(reason: Option<String>, created_by: &Record<User>) > Only<RegistrationToken>
        where "CREATE registration_tokens SET reason = $reason, created_by = $created_by RETURN *"
}

define_relation! {
    RegistrationToken > revoke(id: &Record<RegistrationToken>) > Only<RegistrationToken>
        where "DELETE registration_tokens WHERE id = $id RETURN *"
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize, new)]
pub struct UserCredentials {
    // public claims
    pub exp: i64,

    // user data
    pub id: Record<User>,
    pub username: String,

    // surreal-specific claims
    #[serde(rename = "ns")]
    pub namespace: String,
    #[serde(rename = "db")]
    pub database: String,
    #[serde(rename = "sc")]
    pub scope: String,
    #[serde(rename = "tk")]
    pub token: String,
}

#[derive(Debug, Snafu, Serialize)]
#[serde(tag = "error", content = "data")]
pub enum AuthError {
    #[snafu(display("failed to decode JWT token"))]
    Decode {
        #[serde(skip)]
        source: jsonwebtoken::errors::Error,
        #[serde(skip)]
        #[snafu(implicit)]
        location: Location,
    },

    #[snafu(display("failed to encode JWT token"))]
    Encode {
        #[serde(skip)]
        source: jsonwebtoken::errors::Error,

        #[serde(skip)]
        #[snafu(implicit)]
        location: Location,
    },

    #[snafu(display("request is not authenticated"))]
    ExtractToken {
        #[serde(skip)]
        #[snafu(implicit)]
        location: Location,
    },

    #[snafu(display(
        "cannot sign up with the provided registration token, it may have already been used"
    ))]
    InvalidRegistrationToken {
        token_id: Record<RegistrationToken>,

        #[serde(skip)]
        source: DatabaseQueryError,

        #[serde(skip)]
        #[snafu(implicit)]
        location: Location,
    },

    #[snafu(display("user already exists"))]
    UserAlreadyExists {
        username: String,

        #[serde(skip)]
        source: DatabaseQueryError,

        #[serde(skip)]
        #[snafu(implicit)]
        location: Location,
    },

    #[snafu(display("failed to issue registration token with reason '{reason}'"))]
    IssueRegistrationToken {
        reason: String,

        #[serde(skip)]
        source: DatabaseQueryError,

        #[serde(skip)]
        #[snafu(implicit)]
        location: Location,
    },

    #[snafu(display("invalid login for user '{username}'"))]
    SignIn {
        username: String,

        #[serde(skip)]
        source: DatabaseQueryError,

        #[serde(skip)]
        #[snafu(implicit)]
        location: Location,
    },

    #[snafu(display("unauthorized to issue a new registration token"))]
    RegistrationTokenUnauthorized { user_id: Record<User> },
}

#[derive(Debug, Serialize)]
struct AuthResponse {
    message: String,
    #[serde(flatten)]
    data: AuthError,
}

impl IntoResponse for AuthError {
    fn into_response(self) -> axum::response::Response {
        let content = AuthResponse {
            message: self.to_string(),
            data: self,
        };

        Json(content).into_response()
    }
}

#[derive(Debug, Clone)]
pub struct Authenticator {
    pub secret: SecretString,
    pub algorithm: Algorithm,
    pub validation: Validation,

    pub namespace: String,
    pub database: String,
    pub token_name: String,
    pub scope_name: String,

    pub db: std::sync::Arc<Database>,
}

impl Authenticator {
    fn decoding_key(&self) -> DecodingKey {
        DecodingKey::from_secret(self.secret.expose_secret().as_ref())
    }

    fn encoding_key(&self) -> EncodingKey {
        EncodingKey::from_secret(self.secret.expose_secret().as_ref())
    }

    fn header(&self) -> jsonwebtoken::Header {
        jsonwebtoken::Header::new(self.algorithm)
    }

    pub fn decode(&self, token: &str) -> Result<TokenData<UserCredentials>, AuthError> {
        jsonwebtoken::decode(token, &self.decoding_key(), &self.validation).context(DecodeSnafu)
    }

    pub fn encode(&self, claims: &UserCredentials) -> Result<String, AuthError> {
        jsonwebtoken::encode(&self.header(), claims, &self.encoding_key()).context(EncodeSnafu)
    }

    pub fn expiration(&self) -> i64 {
        (Utc::now() + Duration::days(7)).timestamp()
    }

    pub fn as_credentials(&self, user: &User) -> UserCredentials {
        UserCredentials {
            exp: self.expiration(),

            id: user.id.clone(),
            username: user.username.clone(),

            namespace: self.namespace.clone(),
            database: self.database.clone(),
            scope: self.scope_name.clone(),
            token: self.token_name.clone(),
        }
    }
}

impl Authenticator {
    pub fn extract_token(
        &self,
        request: &Request,
    ) -> Result<TokenData<UserCredentials>, AuthError> {
        let header = request
            .headers()
            .get(header::AUTHORIZATION)
            .context(ExtractTokenSnafu)?;

        let token = header.to_str().ok().context(ExtractTokenSnafu)?;
        let token = token.strip_prefix("Bearer ").context(ExtractTokenSnafu)?;

        self.decode(token)
    }
}

impl Authenticator {
    pub async fn signin(&self, username: &str, password: &str) -> Result<User, AuthError> {
        User::find(username, password, &self.db)
            .await
            .context(SignInSnafu { username })
            .map(|Only(user)| user)
    }

    pub async fn signup(
        &self,
        token_id: Record<RegistrationToken>,
        username: &str,
        password: &str,
    ) -> Result<User, AuthError> {
        let _token = RegistrationToken::revoke(&token_id, &self.db)
            .await
            .context(InvalidRegistrationTokenSnafu { token_id })?;

        let Only(user) = User::create(username, password, &self.db)
            .await
            .context(UserAlreadyExistsSnafu { username })?;

        Ok(user)
    }

    pub async fn issue(
        &self,
        reason: impl AsRef<str>,
        user: &Record<User>,
    ) -> Result<RegistrationToken, AuthError> {
        let reason = reason.as_ref();
        RegistrationToken::issue(Some(reason.into()), user, &self.db)
            .await
            .map(|Only(token)| token)
            .context(IssueRegistrationTokenSnafu { reason })
    }
}

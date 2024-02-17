use std::net::SocketAddr;

use axum::extract::*;
use axum::http::StatusCode;
use axum::routing::{get, post};
use axum::Router;
use http::Method;
use tokio::net::TcpListener;
use tower_http::cors::{Any, CorsLayer};

use crate::{prelude::*, BindAddressSnafu, ServeSnafu};

#[derive(Debug, Clone, FromRef, new)]
pub struct App {
    pub host: SocketAddr,
    pub logger: Logger,
    pub auth: Authenticator,
}

pub type AppState = State<App>;

pub async fn serve(app: App) -> Result<(), InitError> {
    tracing::info!("listening on {}", app.host);
    let listener = TcpListener::bind(app.host)
        .await
        .context(BindAddressSnafu { address: app.host })?;

    let state = app;
    let cors = CorsLayer::new()
        .allow_methods([Method::GET, Method::POST])
        .allow_origin(Any);

    let router = Router::new()
        .route("/generate-token", post(generate_token))
        .route("/signup", post(signup))
        .route("/signin", post(signin))
        .route("/health", get(health))
        .layer(cors)
        .with_state(state);

    axum::serve(listener, router).await.context(ServeSnafu)?;

    Ok(())
}

type Response<T, E> = std::result::Result<Json<T>, (StatusCode, E)>;

async fn health() -> &'static str {
    "ok"
}

#[derive(Debug, Deserialize, Serialize)]
struct Jwt {
    token: String,
    user: User,
}

#[derive(Debug, Deserialize)]
struct SignUpRequest {
    token_id: String,
    username: String,
    password: String,
}

async fn signup(State(app): AppState, Json(body): Json<SignUpRequest>) -> Response<Jwt, AuthError> {
    let token_id = Record::new(body.token_id);
    let auth = &app.auth;

    let user = auth
        .signup(token_id, &body.username, &body.password)
        .await
        .with_code(StatusCode::UNAUTHORIZED)?;

    app.logger.signed_up(&user.id, body.username.clone());

    let claims = auth.as_credentials(&user);
    let token = auth
        .encode(&claims)
        .with_code(StatusCode::INTERNAL_SERVER_ERROR)?;

    let jwt = Jwt { token, user };

    Ok(Json(jwt))
}

#[derive(Debug, Deserialize)]
struct SignInRequest {
    username: String,
    password: String,
}

async fn signin(
    // AppState.auth
    State(auth): State<Authenticator>,
    // Body request
    Json(body): Json<SignInRequest>,
) -> Response<Jwt, AuthError> {
    let user = auth
        .signin(&body.username, &body.password)
        .await
        .with_code(StatusCode::UNAUTHORIZED)?;

    let claims = auth.as_credentials(&user);
    let token = auth
        .encode(&claims)
        .with_code(StatusCode::INTERNAL_SERVER_ERROR)?;

    let jwt = Jwt { token, user };

    Ok(Json(jwt))
}

#[derive(Debug, Deserialize)]
struct GenerateTokenRequest {
    reason: String,
}

async fn generate_token(
    State(auth): State<Authenticator>,
    State(logger): State<Logger>,
    Query(req): Query<GenerateTokenRequest>,
    request: Request,
) -> Result<String, (StatusCode, AuthError)> {
    let token = auth
        .extract_token(&request)
        .with_code(StatusCode::UNAUTHORIZED)?;

    let user_id = token.claims.id;

    let can_generate_token = User::can_generate_token(&user_id, &auth.db)
        .await
        .unwrap_or_default();

    if can_generate_token {
        let token = auth
            .issue(req.reason, &user_id)
            .await
            .with_code(StatusCode::INTERNAL_SERVER_ERROR)?;
        logger.generated_token(&user_id, token.id.clone());

        return Ok(token.id.content());
    }

    Err((
        StatusCode::FORBIDDEN,
        AuthError::RegistrationTokenUnauthorized { user_id },
    ))
}

pub trait ResponseExt<T, E> {
    fn with_code(self, code: StatusCode) -> Result<T, (StatusCode, E)>;
}

impl<T, E> ResponseExt<T, E> for Result<T, E> {
    fn with_code(self, code: StatusCode) -> Result<T, (StatusCode, E)> {
        self.map_err(|err| (code, err))
    }
}

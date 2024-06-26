use axum::response::IntoResponse;
use deadpool_diesel::postgres::Pool;

pub mod api;
pub mod html_views;
pub mod model;
pub mod schema;

#[derive(Clone)]
pub struct AppState {
    pub db_pool: Pool,
    pub flash_config: axum_flash::Config,
}

impl axum::extract::FromRef<AppState> for axum_flash::Config {
    fn from_ref(state: &AppState) -> axum_flash::Config {
        state.flash_config.clone()
    }
}

#[derive(Debug, thiserror::Error)]
pub enum AppError {
    #[error("Pool error: {0}")]
    Pool(#[from] deadpool_diesel::postgres::PoolError),
    #[error("PostgreSQL error: {0}")]
    Diesel(#[from] diesel::result::Error),
    #[error("Deadpool error: {0}")]
    Deadpool(#[from] deadpool_diesel::InteractError),
}

impl IntoResponse for AppError {
    fn into_response(self) -> axum::response::Response {
        (
            axum::http::StatusCode::INTERNAL_SERVER_ERROR,
            "An internal error occurred. Please try again later.",
        )
            .into_response()
    }
}

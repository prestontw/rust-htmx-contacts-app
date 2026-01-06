use axum::response::IntoResponse;
use diesel_async::pooled_connection::deadpool::{self, Pool};
use diesel_async::AsyncPgConnection;

pub mod api;
pub(crate) mod form_struct;
pub mod html_views;
pub(crate) mod hx_triggers;
pub(crate) mod model;
pub(crate) mod schema;

#[derive(Clone)]
pub struct AppState {
    pub db_pool: Pool<AsyncPgConnection>,
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
    Pool(#[from] diesel_async::pooled_connection::PoolError),
    #[error("PostgreSQL error: {0}")]
    Diesel(#[from] diesel::result::Error),
    #[error("Deadpool error: {0}")]
    Deadpool(#[from] deadpool::PoolError),
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

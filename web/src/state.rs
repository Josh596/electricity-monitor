use crate::config::Config;

pub struct AppState {
    pub db: sqlx::SqlitePool,
    pub config: Config,
}

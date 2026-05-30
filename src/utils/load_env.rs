//! Environment loading helpers.

/// Loads environment variables from a local `.env` file when one exists.
pub fn load_env() {
    dotenvy::dotenv().ok();
}

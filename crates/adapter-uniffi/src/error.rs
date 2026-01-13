#[derive(uniffi::Error, Debug, thiserror::Error)]
pub enum Error {
    #[error("Initialization error: {message}")]
    Init { message: String },
}

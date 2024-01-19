use thiserror::Error;
#[derive(Error,Clone,Debug)]
pub enum MQTTError {
    #[error("Default: {0}")]
    Default(String),
    #[error("Received request for thread exit")]
    ExitingThread
}
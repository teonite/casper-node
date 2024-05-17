use crate::crypto::SignerError;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum FinalitySignatureError {
    #[error("signer error: {0}")]
    SignerError(#[from] SignerError),
}

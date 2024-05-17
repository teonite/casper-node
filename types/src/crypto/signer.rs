use crate::{PublicKey, SecretKey, Signature};
use serde::Serialize;
use thiserror::Error;

#[derive(Debug, Error, Serialize)]
pub enum SignerError {
    #[error("public key error: {0}")]
    PublicKey(String),
    #[error("signature error: {0}")]
    Signature(String),
}

pub trait Signer {
    fn public_signing_key(&self) -> PublicKey;

    fn sign_bytes<T: AsRef<[u8]>>(&self, message: T) -> Result<Signature, SignerError>;
}

pub struct TestSigner {}

impl TestSigner {
    pub fn new(secret_key: SecretKey) -> Self {
        todo!()
    }
}

impl Signer for TestSigner {
    fn public_signing_key(&self) -> PublicKey {
        todo!()
    }

    fn sign_bytes<T: AsRef<[u8]>>(&self, message: T) -> Result<Signature, SignerError> {
        todo!()
    }
}

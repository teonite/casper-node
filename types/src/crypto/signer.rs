use thiserror::Error;
use crate::{PublicKey, SecretKey, Signature};

#[derive(Debug, Error)]
pub enum SignerError {
    #[error("public key error: {0}")]
    PublicKey(String),
    #[error("signature error: {0}")]
    Signature(String),
}

pub trait Signer {
    fn public_signing_key(&self) -> Result<PublicKey, SignerError>;

    fn sign<T: AsRef<[u8]>>(&self, message: T) -> Result<Signature, SignerError>;
}

pub struct TestSigner {}

impl TestSigner {
    pub fn new(secret_key: SecretKey) -> Self {
        todo!()
    }
}

impl Signer for TestSigner {
    fn public_signing_key(&self) -> Result<PublicKey, SignerError> {
        todo!()
    }

    fn sign<T: AsRef<[u8]>>(&self, message: T) -> Result<Signature, SignerError> {
        todo!()
    }
}
use std::sync::Arc;

use casper_types::{crypto::Signer, Digest, PublicKey, SecretKey, Signature, SignerError};
use datasize::DataSize;
use thiserror::Error;

use crate::{consensus::ValidatorSecret, utils::specimen::LargestSpecimen};

#[derive(Error, Debug)]
pub enum NodeSignerError {
    #[error("Failed to setup node signer")]
    SetupError,
}

#[derive(Clone, DataSize)]
pub enum NodeSigner {
    Local(LocalSigner),
    Remote(RemoteSigner),
}

impl NodeSigner {
    /// Creates a local signer for `MockReactor`.
    pub fn mock(secret_key: SecretKey) -> Arc<Self> {
        todo!()
    }
}

impl Signer for NodeSigner {
    fn public_signing_key(&self) -> Result<PublicKey, SignerError> {
        todo!()
    }

    fn sign<T: AsRef<[u8]>>(&self, message: T) -> Result<Signature, SignerError> {
        todo!()
    }
}

impl ValidatorSecret for NodeSigner {
    type Hash = Digest;
    type Signature = Signature;

    fn sign(&self, hash: &Self::Hash) -> Self::Signature {
        todo!()
    }
}

impl LargestSpecimen for NodeSigner {
    fn largest_specimen<E: crate::utils::specimen::SizeEstimator>(
        estimator: &E,
        cache: &mut crate::utils::specimen::Cache,
    ) -> Self {
        todo!()
    }
}

/// Signer using key files stored on local filesystem.
#[derive(Clone, DataSize)]
pub struct LocalSigner {}

impl LocalSigner {
    pub fn new() -> Self {
        // let (our_secret_key, our_public_key) = config.consensus.load_keys(&root_dir)?;
        todo!()
    }
}

/// Signer using remote HTTP signing service.
#[derive(Clone, DataSize)]
pub struct RemoteSigner {}

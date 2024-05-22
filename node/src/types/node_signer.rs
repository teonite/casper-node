use std::sync::Arc;

use casper_types::{crypto, crypto::Signer, Digest, ErrorExt, PublicKey, SecretKey, Signature};
use datasize::DataSize;
use serde::Serialize;
use thiserror::Error;

use crate::{
    consensus::ValidatorSecret,
    utils::{specimen::LargestSpecimen, LoadError},
};

#[derive(Error, Debug, Serialize)]
pub enum NodeSignerError {
    #[error("Failed to setup node signer")]
    SetupError,

    #[error(transparent)]
    Crypto(#[from] crypto::Error),

    #[error("Failed to load private key from file")]
    LoadKeyError(
        #[serde(skip_serializing)]
        #[source]
        LoadError<ErrorExt>,
    ),
}

#[derive(DataSize)]
pub enum NodeSigner {
    Local(LocalSigner),
    Remote(RemoteSigner),
}

impl NodeSigner {
    /// Creates a local signer for `MockReactor`.
    pub fn mock(secret_key: SecretKey) -> Arc<Self> {
        let public_key = PublicKey::from(&secret_key);
        Self::local(secret_key, public_key)
    }

    /// Creates an instance of local node signer.
    pub fn local(secret_key: SecretKey, public_key: PublicKey) -> Arc<Self> {
        let local_signer = LocalSigner::new(secret_key, public_key);
        Arc::new(Self::Local(local_signer))
    }
}

impl Signer for NodeSigner {
    fn public_signing_key(&self) -> PublicKey {
        match self {
            NodeSigner::Local(local_signer) => local_signer.public_key.clone(),
            NodeSigner::Remote(_remote_signer) => {
                unimplemented!()
            }
        }
    }

    fn sign_bytes<T: AsRef<[u8]>>(&self, message: T) -> Result<Signature, crypto::Error> {
        match self {
            NodeSigner::Local(local_signer) => Ok(crypto::sign(
                message,
                &local_signer.secret_key,
                &local_signer.public_key,
            )),
            NodeSigner::Remote(_remote_signer) => {
                unimplemented!()
            }
        }
    }
}

impl ValidatorSecret for Arc<NodeSigner> {
    type Hash = Digest;
    type Signature = Signature;

    fn sign(&self, hash: &Self::Hash) -> Self::Signature {
        self.sign_bytes(hash).unwrap()
    }
}

impl LargestSpecimen for NodeSigner {
    fn largest_specimen<E: crate::utils::specimen::SizeEstimator>(
        _estimator: &E,
        _cache: &mut crate::utils::specimen::Cache,
    ) -> Self {
        unimplemented!()
    }
}

/// Signer using key files stored on local filesystem.
#[derive(DataSize)]
pub struct LocalSigner {
    public_key: PublicKey,
    secret_key: SecretKey,
}

impl LocalSigner {
    pub fn new(secret_key: SecretKey, public_key: PublicKey) -> Self {
        LocalSigner {
            public_key,
            secret_key,
        }
    }
}

/// Signer using remote HTTP signing service.
#[derive(DataSize)]
pub struct RemoteSigner {
    public_key: PublicKey,
}

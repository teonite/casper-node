use std::sync::Arc;

use casper_types::{
    crypto, BlockV2, ChainNameDigest, Digest, ErrorExt, FinalitySignatureV2, PublicKey, SecretKey,
    Signature,
};
use datasize::DataSize;
use reqwest::Client;
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

    pub fn public_signing_key(&self) -> PublicKey {
        match self {
            NodeSigner::Local(local_signer) => local_signer.public_key.clone(),
            NodeSigner::Remote(remote_signer) => remote_signer.public_key.clone(),
        }
    }

    pub async fn get_signature<T: AsRef<[u8]>>(
        &self,
        bytes: T,
    ) -> Result<Signature, crypto::Error> {
        match self {
            NodeSigner::Local(local_signer) => Ok(local_signer.sign(bytes)),
            NodeSigner::Remote(_remote_signer) => {
                unimplemented!()
            }
        }
    }

    /// Generate a signature from local signer as a blocking operation.
    #[cfg(any(feature = "testing", test))]
    pub fn get_signature_sync<T: AsRef<[u8]>>(&self, bytes: T) -> Signature {
        match self {
            NodeSigner::Local(local_signer) => local_signer.sign(bytes),
            NodeSigner::Remote(_) => {
                panic!("test signature generation shouldn't be called for a remote signer")
            }
        }
    }

    #[cfg(any(feature = "testing", test))]
    pub fn create_finality_signature(
        &self,
        block: Arc<BlockV2>,
        chain_name_hash: ChainNameDigest,
    ) -> FinalitySignatureV2 {
        match self {
            NodeSigner::Local(local_signer) => {
                let block_hash = *block.hash();
                let block_height = block.height();
                let era_id = block.era_id();
                let signature = local_signer.sign(FinalitySignatureV2::bytes_to_sign(
                    block_hash,
                    block_height,
                    era_id,
                    chain_name_hash,
                ));
                FinalitySignatureV2::new(
                    block_hash,
                    block_height,
                    era_id,
                    chain_name_hash,
                    signature,
                    self.public_signing_key(),
                )
            }
            NodeSigner::Remote(_) => {
                panic!("test signature generation shouldn't be called for a remote signer")
            }
        }
    }
}

impl ValidatorSecret for Arc<NodeSigner> {
    type Hash = Digest;
    type Signature = Signature;

    #[cfg(any(feature = "testing", test))]
    fn sign(&self, hash: &Self::Hash) -> Self::Signature {
        self.get_signature_sync(hash)
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

    pub fn sign<T: AsRef<[u8]>>(&self, bytes: T) -> Signature {
        crypto::sign(bytes, &self.secret_key, &self.public_key)
    }
}

/// Signer using remote HTTP signing service.
#[derive(DataSize)]
pub struct RemoteSigner {
    public_key: PublicKey,
    #[data_size(skip)] // FIXME: add size estimate
    client: Client,
}

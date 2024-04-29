use std::sync::Arc;

use casper_types::{PublicKey, SecretKey, Signature};
use datasize::DataSize;
use thiserror::Error;

#[derive(Error, Debug)]
pub(crate) enum NodeSignerError {
    #[error("Failed to setup node signer")]
    SetupError,
}

#[derive(Clone, DataSize)]
pub(crate) enum NodeSigner {
    Local(LocalSigner),
    Remote(RemoteSigner),
}

impl NodeSigner {
    /// Creates a local signer for `MockReactor`.
    pub(crate) fn mock(secret_key: Arc<SecretKey>) -> Self {
        unimplemented!()
    }
}

/// Signer using key files stored on local filesystem.
#[derive(Clone, DataSize)]
pub(crate) struct LocalSigner {}

impl LocalSigner {
    pub(crate) fn new() -> Self {
        // let (our_secret_key, our_public_key) = config.consensus.load_keys(&root_dir)?;
        todo!()
    }
}

/// Signer using remote HTTP signing service.
#[derive(Clone, DataSize)]
pub(crate) struct RemoteSigner {}

impl NodeSigner {
    fn pubkey() -> Result<PublicKey, NodeSignerError> {
        todo!()
    }

    fn sign() -> Result<Signature, NodeSignerError> {
        todo!()
    }
}

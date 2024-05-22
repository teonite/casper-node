use crate::{crypto, PublicKey, SecretKey, Signature};

pub trait Signer {
    fn public_signing_key(&self) -> PublicKey;

    fn sign_bytes<T: AsRef<[u8]>>(&self, message: T) -> Result<Signature, crypto::Error>;
}

pub struct TestSigner {
    public_key: PublicKey,
    secret_key: SecretKey,
}

impl TestSigner {
    pub fn new(secret_key: SecretKey) -> Self {
        let public_key = PublicKey::from(&secret_key);
        TestSigner {
            public_key,
            secret_key,
        }
    }
}

impl Signer for TestSigner {
    fn public_signing_key(&self) -> PublicKey {
        self.public_key.clone()
    }

    fn sign_bytes<T: AsRef<[u8]>>(&self, message: T) -> Result<Signature, crypto::Error> {
        Ok(crypto::sign(message, &self.secret_key, &self.public_key))
    }
}

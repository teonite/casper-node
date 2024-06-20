use alloc::vec::Vec;
use core::fmt::{self, Display, Formatter};

#[cfg(feature = "datasize")]
use datasize::DataSize;
#[cfg(feature = "json-schema")]
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::{
    bytesrepr::{self, FromBytes, ToBytes},
    CLType, CLTyped, PublicKey,
};

/// Represents a party delegating their stake to a validator (or "delegatee")
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "datasize", derive(DataSize))]
#[cfg_attr(feature = "json-schema", derive(JsonSchema))]
#[serde(deny_unknown_fields)]
pub struct WhitelistEntry {
    delegator_public_key: PublicKey,
    // staked_amount: U512,
    // bonding_purse: URef,
    validator_public_key: PublicKey,
    // vesting_schedule: Option<VestingSchedule>,
}

impl WhitelistEntry {
    /// Creates a new [`WhitelistEntry`]
    pub fn new(delegator_public_key: PublicKey, validator_public_key: PublicKey) -> Self {
        Self {
            delegator_public_key,
            validator_public_key,
        }
    }

    /// Returns public key of the delegator.
    pub fn delegator_public_key(&self) -> &PublicKey {
        &self.delegator_public_key
    }

    /// Returns delegatee
    pub fn validator_public_key(&self) -> &PublicKey {
        &self.validator_public_key
    }
}

impl CLTyped for WhitelistEntry {
    fn cl_type() -> CLType {
        CLType::Any
    }
}

impl ToBytes for WhitelistEntry {
    fn to_bytes(&self) -> Result<Vec<u8>, bytesrepr::Error> {
        let mut buffer = bytesrepr::allocate_buffer(self)?;
        buffer.extend(self.delegator_public_key.to_bytes()?);
        buffer.extend(self.validator_public_key.to_bytes()?);
        Ok(buffer)
    }

    fn serialized_length(&self) -> usize {
        self.delegator_public_key.serialized_length()
            + self.validator_public_key.serialized_length()
    }

    fn write_bytes(&self, writer: &mut Vec<u8>) -> Result<(), bytesrepr::Error> {
        self.delegator_public_key.write_bytes(writer)?;
        self.validator_public_key.write_bytes(writer)?;
        Ok(())
    }
}

impl FromBytes for WhitelistEntry {
    fn from_bytes(bytes: &[u8]) -> Result<(Self, &[u8]), bytesrepr::Error> {
        let (delegator_public_key, bytes) = PublicKey::from_bytes(bytes)?;
        let (validator_public_key, bytes) = PublicKey::from_bytes(bytes)?;
        Ok((
            Self {
                delegator_public_key,
                validator_public_key,
            },
            bytes,
        ))
    }
}

impl Display for WhitelistEntry {
    fn fmt(&self, formatter: &mut Formatter) -> fmt::Result {
        write!(
            formatter,
            "whitelist entry {{ delegator {}, validator {} }}",
            self.delegator_public_key, self.validator_public_key
        )
    }
}

#[cfg(test)]
mod tests {
    use crate::{bytesrepr, system::auction::WhitelistEntry, PublicKey, SecretKey};

    #[test]
    fn serialization_roundtrip() {
        let delegator_public_key: PublicKey = PublicKey::from(
            &SecretKey::ed25519_from_bytes([42; SecretKey::ED25519_LENGTH]).unwrap(),
        );

        let validator_public_key: PublicKey = PublicKey::from(
            &SecretKey::ed25519_from_bytes([43; SecretKey::ED25519_LENGTH]).unwrap(),
        );
        let entry = WhitelistEntry::new(delegator_public_key, validator_public_key);
        bytesrepr::test_serialization_roundtrip(&entry);
    }
}

#[cfg(test)]
mod prop_tests {
    use proptest::prelude::*;

    use crate::{bytesrepr, gens};

    proptest! {
        #[test]
        fn test_value_bid(bid in gens::whitelist_entry_arb()) {
            bytesrepr::test_serialization_roundtrip(&bid);
        }
    }
}

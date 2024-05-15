//! Costs of the `entity` system contract.
#[cfg(feature = "datasize")]
use datasize::DataSize;
#[cfg(any(feature = "testing", test))]
use rand::{
    distributions::{Distribution, Standard},
    Rng,
};
use serde::{Deserialize, Serialize};

use crate::bytesrepr::{self, FromBytes, ToBytes};

/// Default cost of the `add_associated_key` entity entry point.
pub const DEFAULT_ADD_ASSOCIATED_COST: u64 = 10_000;
/// Default cost of the `remove_associated_key` entity entry point.
pub const DEFAULT_REMOVE_ASSOCIATED_COST: u64 = 10_000;
/// Default cost of the `update_associated_key` entity entry point.
pub const DEFAULT_UPDATE_ASSOCIATED_COST: u64 = 10_000;

/// Description of the costs of calling `entity` entrypoints.
#[derive(Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Debug)]
#[cfg_attr(feature = "datasize", derive(DataSize))]
#[serde(deny_unknown_fields)]
pub struct EntityCosts {
    /// Cost of calling the `add_associated_key` entry point.
    pub add_associated_key: u64,
    /// Cost of calling the `remove_associated_key` entry point.
    pub remove_associated_key: u64,
    /// Cost of calling the `update_associated_key` entry point.
    pub update_associated_key: u64,
}

impl Default for EntityCosts {
    fn default() -> Self {
        Self {
            add_associated_key: DEFAULT_ADD_ASSOCIATED_COST,
            remove_associated_key: DEFAULT_REMOVE_ASSOCIATED_COST,
            update_associated_key: DEFAULT_UPDATE_ASSOCIATED_COST,
        }
    }
}

impl ToBytes for EntityCosts {
    fn to_bytes(&self) -> Result<Vec<u8>, bytesrepr::Error> {
        let mut ret = bytesrepr::unchecked_allocate_buffer(self);

        ret.append(&mut self.add_associated_key.to_bytes()?);
        ret.append(&mut self.remove_associated_key.to_bytes()?);
        ret.append(&mut self.update_associated_key.to_bytes()?);

        Ok(ret)
    }

    fn serialized_length(&self) -> usize {
        self.add_associated_key.serialized_length()
            + self.remove_associated_key.serialized_length()
            + self.update_associated_key.serialized_length()
    }
}

impl FromBytes for EntityCosts {
    fn from_bytes(bytes: &[u8]) -> Result<(Self, &[u8]), bytesrepr::Error> {
        let (add_associated_key, rem) = FromBytes::from_bytes(bytes)?;
        let (remove_associated_key, rem) = FromBytes::from_bytes(rem)?;
        let (update_associated_key, rem) = FromBytes::from_bytes(rem)?;

        Ok((
            Self {
                add_associated_key,
                remove_associated_key,
                update_associated_key,
            },
            rem,
        ))
    }
}

#[cfg(any(feature = "testing", test))]
impl Distribution<EntityCosts> for Standard {
    fn sample<R: Rng + ?Sized>(&self, rng: &mut R) -> EntityCosts {
        EntityCosts {
            add_associated_key: rng.gen::<u32>() as u64,
            remove_associated_key: rng.gen::<u32>() as u64,
            update_associated_key: rng.gen::<u32>() as u64,
        }
    }
}

#[doc(hidden)]
#[cfg(any(feature = "gens", test))]
pub mod gens {
    use proptest::{num, prop_compose};

    use super::EntityCosts;

    prop_compose! {
        pub fn entity_costs_arb()(
            add_associated_key in num::u64::ANY,
            remove_associated_key in num::u64::ANY,
            update_associated_key in num::u64::ANY,
        ) -> EntityCosts {
            EntityCosts {
                add_associated_key,
                remove_associated_key,
                update_associated_key,
            }
        }
    }
}
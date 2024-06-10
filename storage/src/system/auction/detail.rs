use std::{
    collections::{BTreeMap, BTreeSet, HashSet},
    convert::TryInto,
    ops::Mul,
};

use num_rational::Ratio;

use casper_types::{
    account::AccountHash,
    bytesrepr::{FromBytes, ToBytes},
    system::auction::{
        BidAddr, BidKind, Delegator, Error, SeigniorageAllocation, SeigniorageRecipient,
        SeigniorageRecipientsSnapshot, UnbondingPurse, UnbondingPurses, ValidatorBid,
        ValidatorBids, ValidatorCredit, ValidatorCredits, AUCTION_DELAY_KEY,
        ERA_END_TIMESTAMP_MILLIS_KEY, ERA_ID_KEY, SEIGNIORAGE_RECIPIENTS_SNAPSHOT_KEY,
        UNBONDING_DELAY_KEY, VALIDATOR_SLOTS_KEY,
    },
    ApiError, CLTyped, EraId, Key, KeyTag, PublicKey, URef, U512,
};
use tracing::{debug, error, warn};

use super::{
    Auction, EraValidators, MintProvider, RuntimeProvider, StorageProvider, ValidatorWeights,
};

/// Maximum length of bridge records chain.
/// Used when looking for the most recent bid record to avoid unbounded computations.
const MAX_BRIDGE_CHAIN_LENGTH: u64 = 20;

fn read_from<P, T>(provider: &mut P, name: &str) -> Result<T, Error>
where
    P: StorageProvider + RuntimeProvider + ?Sized,
    T: FromBytes + CLTyped,
{
    let key = match provider.named_keys_get(name) {
        None => {
            error!("auction missing named key {:?}", name);
            return Err(Error::MissingKey);
        }
        Some(key) => key,
    };
    let uref = key.into_uref().ok_or(Error::InvalidKeyVariant)?;
    let value: T = provider.read(uref)?.ok_or(Error::MissingValue)?;
    Ok(value)
}

fn write_to<P, T>(provider: &mut P, name: &str, value: T) -> Result<(), Error>
where
    P: StorageProvider + RuntimeProvider + ?Sized,
    T: ToBytes + CLTyped,
{
    let key = provider.named_keys_get(name).ok_or(Error::MissingKey)?;
    let uref = key.into_uref().ok_or(Error::InvalidKeyVariant)?;
    provider.write(uref, value)
}

#[derive(Debug, Default)]
pub struct ValidatorBidsDetail {
    validator_bids: ValidatorBids,
    validator_credits: ValidatorCredits,
}

impl ValidatorBidsDetail {
    pub fn new() -> Self {
        ValidatorBidsDetail {
            validator_bids: BTreeMap::new(),
            validator_credits: BTreeMap::new(),
        }
    }

    /// Inserts a validator bid.
    pub fn insert_bid(
        &mut self,
        validator: PublicKey,
        validator_bid: Box<ValidatorBid>,
    ) -> Option<Box<ValidatorBid>> {
        self.validator_bids.insert(validator, validator_bid)
    }

    /// Inserts a validator credit.
    pub fn insert_credit(
        &mut self,
        validator: PublicKey,
        era_id: EraId,
        validator_credit: Box<ValidatorCredit>,
    ) {
        let credits = &mut self.validator_credits;

        credits
            .entry(validator.clone())
            .and_modify(|inner| {
                inner
                    .entry(era_id)
                    .and_modify(|_| {
                        warn!(
                            ?validator,
                            ?era_id,
                            "multiple validator credit entries in same era"
                        )
                    })
                    .or_insert(validator_credit.clone());
            })
            .or_insert_with(|| {
                let mut inner = BTreeMap::new();
                inner.insert(era_id, validator_credit);
                inner
            });
    }

    /// Get validator weights.
    #[allow(clippy::too_many_arguments)]
    pub fn validator_weights<P>(
        &mut self,
        provider: &mut P,
        era_ending: EraId,
        era_end_timestamp_millis: u64,
        vesting_schedule_period_millis: u64,
        locked: bool,
        include_credits: bool,
        cap: Ratio<U512>,
    ) -> Result<ValidatorWeights, Error>
    where
        P: RuntimeProvider + ?Sized + StorageProvider,
    {
        let mut ret = BTreeMap::new();

        for (validator_public_key, bid) in self.validator_bids.iter().filter(|(_, v)| {
            locked
                == v.is_locked_with_vesting_schedule(
                    era_end_timestamp_millis,
                    vesting_schedule_period_millis,
                )
                && !v.inactive()
        }) {
            let staked_amount = total_staked_amount(provider, bid)?;
            let credit_amount = self.credit_amount(
                validator_public_key,
                era_ending,
                staked_amount,
                include_credits,
                cap,
            );
            let total = staked_amount.saturating_add(credit_amount);
            ret.insert(validator_public_key.clone(), total);
        }

        Ok(ret)
    }

    fn credit_amount(
        &self,
        validator_public_key: &PublicKey,
        era_ending: EraId,
        staked_amount: U512,
        include_credit: bool,
        cap: Ratio<U512>,
    ) -> U512 {
        if !include_credit {
            return U512::zero();
        }

        if let Some(inner) = self.validator_credits.get(validator_public_key) {
            if let Some(credit) = inner.get(&era_ending) {
                let capped = Ratio::new_raw(staked_amount, U512::one())
                    .mul(cap)
                    .to_integer();
                let credit_amount = credit.amount();
                return credit_amount.min(capped);
            }
        }

        U512::zero()
    }

    pub(crate) fn validator_bids_mut(&mut self) -> &mut ValidatorBids {
        &mut self.validator_bids
    }

    /// Consume self into in underlying collections.
    pub fn destructure(self) -> (ValidatorBids, ValidatorCredits) {
        (self.validator_bids, self.validator_credits)
    }
}

/// Prunes away all validator credits for the imputed era, which should be the era ending.
///
/// This is intended to be called at the end of an era, after calculating validator weights.
pub fn prune_validator_credits<P>(
    provider: &mut P,
    era_ending: EraId,
    validator_credits: ValidatorCredits,
) where
    P: StorageProvider + RuntimeProvider + ?Sized,
{
    for (validator_public_key, inner) in validator_credits {
        if inner.contains_key(&era_ending) {
            provider.prune_bid(BidAddr::new_credit(&validator_public_key, era_ending))
        }
    }
}

pub fn get_validator_bids<P>(provider: &mut P, era_id: EraId) -> Result<ValidatorBidsDetail, Error>
where
    P: StorageProvider + RuntimeProvider + ?Sized,
{
    let bids_keys = provider.get_keys(&KeyTag::BidAddr)?;

    let mut ret = ValidatorBidsDetail::new();

    for key in bids_keys {
        match provider.read_bid(&key)? {
            Some(BidKind::Validator(validator_bid)) => {
                ret.insert_bid(validator_bid.validator_public_key().clone(), validator_bid);
            }
            Some(BidKind::Credit(credit)) => {
                ret.insert_credit(credit.validator_public_key().clone(), era_id, credit);
            }
            Some(_) => {
                // noop
            }
            None => return Err(Error::ValidatorNotFound),
        };
    }

    Ok(ret)
}

pub fn set_validator_bids<P>(provider: &mut P, validators: ValidatorBids) -> Result<(), Error>
where
    P: StorageProvider + RuntimeProvider + ?Sized,
{
    for (validator_public_key, validator_bid) in validators.into_iter() {
        let bid_addr = BidAddr::from(validator_public_key.clone());
        provider.write_bid(bid_addr.into(), BidKind::Validator(validator_bid))?;
    }
    Ok(())
}

pub fn get_unbonding_purses<P>(provider: &mut P) -> Result<UnbondingPurses, Error>
where
    P: StorageProvider + RuntimeProvider + ?Sized,
{
    let unbond_keys = provider.get_keys(&KeyTag::Unbond)?;

    let mut ret = BTreeMap::new();

    for key in unbond_keys {
        let account_hash = match key {
            Key::Unbond(account_hash) => account_hash,
            _ => return Err(Error::InvalidKeyVariant),
        };
        let unbonding_purses = provider.read_unbonds(&account_hash)?;
        ret.insert(account_hash, unbonding_purses);
    }

    Ok(ret)
}

pub fn set_unbonding_purses<P>(
    provider: &mut P,
    unbonding_purses: UnbondingPurses,
) -> Result<(), Error>
where
    P: StorageProvider + RuntimeProvider + ?Sized,
{
    for (account_hash, unbonding_purses) in unbonding_purses.into_iter() {
        provider.write_unbonds(account_hash, unbonding_purses)?;
    }
    Ok(())
}

pub fn get_era_id<P>(provider: &mut P) -> Result<EraId, Error>
where
    P: StorageProvider + RuntimeProvider + ?Sized,
{
    read_from(provider, ERA_ID_KEY)
}

pub fn set_era_id<P>(provider: &mut P, era_id: EraId) -> Result<(), Error>
where
    P: StorageProvider + RuntimeProvider + ?Sized,
{
    write_to(provider, ERA_ID_KEY, era_id)
}

pub fn get_era_end_timestamp_millis<P>(provider: &mut P) -> Result<u64, Error>
where
    P: StorageProvider + RuntimeProvider + ?Sized,
{
    read_from(provider, ERA_END_TIMESTAMP_MILLIS_KEY)
}

pub fn set_era_end_timestamp_millis<P>(
    provider: &mut P,
    era_end_timestamp_millis: u64,
) -> Result<(), Error>
where
    P: StorageProvider + RuntimeProvider + ?Sized,
{
    write_to(
        provider,
        ERA_END_TIMESTAMP_MILLIS_KEY,
        era_end_timestamp_millis,
    )
}

pub fn get_seigniorage_recipients_snapshot<P>(
    provider: &mut P,
) -> Result<SeigniorageRecipientsSnapshot, Error>
where
    P: StorageProvider + RuntimeProvider + ?Sized,
{
    read_from(provider, SEIGNIORAGE_RECIPIENTS_SNAPSHOT_KEY)
}

pub fn set_seigniorage_recipients_snapshot<P>(
    provider: &mut P,
    snapshot: SeigniorageRecipientsSnapshot,
) -> Result<(), Error>
where
    P: StorageProvider + RuntimeProvider + ?Sized,
{
    write_to(provider, SEIGNIORAGE_RECIPIENTS_SNAPSHOT_KEY, snapshot)
}

pub fn get_validator_slots<P>(provider: &mut P) -> Result<usize, Error>
where
    P: StorageProvider + RuntimeProvider + ?Sized,
{
    let validator_slots: u32 = match read_from(provider, VALIDATOR_SLOTS_KEY) {
        Ok(ret) => ret,
        Err(err) => {
            error!("Failed to find VALIDATOR_SLOTS_KEY {}", err);
            return Err(err);
        }
    };
    let validator_slots = validator_slots
        .try_into()
        .map_err(|_| Error::InvalidValidatorSlotsValue)?;
    Ok(validator_slots)
}

pub fn get_auction_delay<P>(provider: &mut P) -> Result<u64, Error>
where
    P: StorageProvider + RuntimeProvider + ?Sized,
{
    let auction_delay: u64 = match read_from(provider, AUCTION_DELAY_KEY) {
        Ok(ret) => ret,
        Err(err) => {
            error!("Failed to find AUCTION_DELAY_KEY {}", err);
            return Err(err);
        }
    };
    Ok(auction_delay)
}

fn get_unbonding_delay<P>(provider: &mut P) -> Result<u64, Error>
where
    P: StorageProvider + RuntimeProvider + ?Sized,
{
    read_from(provider, UNBONDING_DELAY_KEY)
}

/// Iterates over unbonding entries and checks if a locked amount can be paid already if
/// a specific era is reached.
///
/// This function can be called by the system only.
pub fn process_unbond_requests<P: Auction + ?Sized>(
    provider: &mut P,
    max_delegators_per_validator: u32,
    minimum_delegation_amount: u64,
) -> Result<(), ApiError> {
    if provider.get_caller() != PublicKey::System.to_account_hash() {
        return Err(Error::InvalidCaller.into());
    }

    // Update `unbonding_purses` data
    let mut unbonding_purses: UnbondingPurses = get_unbonding_purses(provider)?;

    let current_era_id = provider.read_era_id()?;

    let unbonding_delay = get_unbonding_delay(provider)?;

    let mut processed_validators = BTreeSet::new();
    let mut still_unbonding_public_keys = HashSet::new();

    for unbonding_list in unbonding_purses.values_mut() {
        let mut new_unbonding_list = Vec::new();

        for unbonding_purse in unbonding_list.iter() {
            // Since `process_unbond_requests` is run before `run_auction`, we should check if
            // current era id + unbonding delay is equal or greater than the `era_of_creation` that
            // was calculated on `unbond` attempt.
            if current_era_id >= unbonding_purse.era_of_creation() + unbonding_delay {
                // remember the validator's key, so that we can later check if we can prune their
                // bid now that the unbond has been processed
                processed_validators.insert(unbonding_purse.validator_public_key().clone());
                match handle_redelegation(
                    provider,
                    unbonding_purse,
                    max_delegators_per_validator,
                    minimum_delegation_amount,
                )? {
                    UnbondRedelegationOutcome::SuccessfullyRedelegated => {
                        // noop; on successful redelegation, no actual unbond occurs
                    }
                    uro @ UnbondRedelegationOutcome::NonexistantRedelegationTarget
                    | uro @ UnbondRedelegationOutcome::DelegationAmountBelowCap
                    | uro @ UnbondRedelegationOutcome::RedelegationTargetHasNoVacancy
                    | uro @ UnbondRedelegationOutcome::RedelegationTargetIsUnstaked
                    | uro @ UnbondRedelegationOutcome::Withdrawal => {
                        // Move funds from bid purse to unbonding purse
                        provider.unbond(unbonding_purse).map_err(|err| {
                            error!("Error unbonding purse {err:?} ({uro:?})");
                            ApiError::from(Error::TransferToUnbondingPurse)
                        })?
                    }
                }
            } else {
                new_unbonding_list.push(unbonding_purse.clone());
                // remember the key of the unbonder that is still not fully unbonded - so that we
                // don't prune bids of validators that still have delegators waiting for unbonding,
                // or the waiting delegators
                still_unbonding_public_keys.insert(unbonding_purse.unbonder_public_key().clone());
            }
        }
        *unbonding_list = new_unbonding_list;
    }

    // revisit the validators for which we processed some unbonds and see if we can now prune their
    // or their delegators' bids
    for validator_public_key in processed_validators {
        let validator_bid_addr = BidAddr::new_from_public_keys(&validator_public_key, None);
        let validator_bid = read_current_validator_bid(provider, validator_bid_addr.into())?;
        let validator_public_key = validator_bid.validator_public_key().clone(); // use the current
                                                                                 // public key
        let mut delegator_bids = read_delegator_bids(provider, &validator_public_key)?;

        // prune the delegators that have no stake and no remaining unbonds to be processed
        delegator_bids.retain(|delegator| {
            if delegator.staked_amount().is_zero()
                && !still_unbonding_public_keys.contains(delegator.delegator_public_key())
            {
                let delegator_bid_addr = BidAddr::new_from_public_keys(
                    &validator_public_key,
                    Some(delegator.delegator_public_key()),
                );
                debug!("pruning delegator bid {}", delegator_bid_addr);
                provider.prune_bid(delegator_bid_addr);
                false
            } else {
                true
            }
        });

        // if the validator has no delegators, no stake and no remaining unbonds, prune them, too
        if !still_unbonding_public_keys.contains(&validator_public_key)
            && delegator_bids.is_empty()
            && validator_bid.staked_amount().is_zero()
        {
            debug!("pruning validator bid {}", validator_bid_addr);
            provider.prune_bid(validator_bid_addr);
        }
    }

    set_unbonding_purses(provider, unbonding_purses)?;
    Ok(())
}

/// Creates a new purse in unbonding_purses given a validator's key, amount, and a destination
/// unbonding purse. Returns the amount of motes remaining in the validator's bid purse.
pub fn create_unbonding_purse<P: Auction + ?Sized>(
    provider: &mut P,
    validator_public_key: PublicKey,
    unbonder_public_key: PublicKey,
    bonding_purse: URef,
    amount: U512,
    new_validator: Option<PublicKey>,
) -> Result<(), Error> {
    if provider
        .available_balance(bonding_purse)?
        .unwrap_or_default()
        < amount
    {
        return Err(Error::UnbondTooLarge);
    }

    let account_hash = AccountHash::from(&unbonder_public_key);
    let mut unbonding_purses = provider.read_unbonds(&account_hash)?;
    let era_of_creation = provider.read_era_id()?;
    let new_unbonding_purse = UnbondingPurse::new(
        bonding_purse,
        validator_public_key,
        unbonder_public_key,
        era_of_creation,
        amount,
        new_validator,
    );
    unbonding_purses.push(new_unbonding_purse);
    provider.write_unbonds(account_hash, unbonding_purses)?;

    Ok(())
}

/// Returns most recent validator public key if public key has been changed
/// or the validator has withdrawn their bid completely.
pub fn get_most_recent_validator_public_key<P>(
    provider: &mut P,
    mut validator_public_key: PublicKey,
) -> Result<PublicKey, Error>
where
    P: RuntimeProvider + StorageProvider,
{
    let mut validator_bid_addr = BidAddr::from(validator_public_key.clone());
    let mut found_validator_bid_chain_tip = false;
    for _ in 0..MAX_BRIDGE_CHAIN_LENGTH {
        match provider.read_bid(&validator_bid_addr.into())? {
            Some(BidKind::Validator(validator_bid)) => {
                validator_public_key = validator_bid.validator_public_key().clone();
                found_validator_bid_chain_tip = true;
                break;
            }
            Some(BidKind::Bridge(bridge)) => {
                validator_public_key = bridge.new_validator_public_key().clone();
                validator_bid_addr = BidAddr::from(validator_public_key.clone());
            }
            _ => {
                // Validator has withdrawn their bid, so there's nothing at the tip.
                // In this case we add the reward to a delegator's unbond.
                found_validator_bid_chain_tip = true;
                break;
            }
        };
    }
    if !found_validator_bid_chain_tip {
        Err(Error::BridgeRecordChainTooLong)
    } else {
        Ok(validator_public_key)
    }
}

/// Attempts to apply the delegator reward to the existing stake. If the reward recipient has
/// completely unstaked, applies it to their unbond instead. In either case, returns
/// the purse the amount should be applied to.
pub fn distribute_delegator_rewards<P>(
    provider: &mut P,
    seigniorage_allocations: &mut Vec<SeigniorageAllocation>,
    validator_public_key: PublicKey,
    rewards: impl IntoIterator<Item = (PublicKey, U512)>,
) -> Result<Vec<(AccountHash, U512, URef)>, Error>
where
    P: RuntimeProvider + StorageProvider,
{
    let mut delegator_payouts = Vec::new();
    for (delegator_public_key, delegator_reward_trunc) in rewards {
        let bid_key =
            BidAddr::new_from_public_keys(&validator_public_key, Some(&delegator_public_key))
                .into();

        let delegator_bonding_purse = match read_delegator_bid(provider, &bid_key) {
            Ok(mut delegator_bid) if !delegator_bid.staked_amount().is_zero() => {
                let purse = *delegator_bid.bonding_purse();
                delegator_bid.increase_stake(delegator_reward_trunc)?;
                provider.write_bid(bid_key, BidKind::Delegator(delegator_bid))?;
                purse
            }
            Ok(_) | Err(Error::DelegatorNotFound) => {
                // check to see if there are unbond entries for this recipient
                // (validator + delegator match), and if there are apply the amount
                // to the unbond entry with the highest era.
                let account_hash = delegator_public_key.to_account_hash();
                match provider.read_unbonds(&account_hash) {
                    Ok(mut unbonds) => {
                        match unbonds
                            .iter_mut()
                            .filter(|x| x.validator_public_key() == &validator_public_key)
                            .max_by(|x, y| x.era_of_creation().cmp(&y.era_of_creation()))
                        {
                            Some(unbond) => {
                                let purse = *unbond.bonding_purse();
                                let new_amount =
                                    unbond.amount().saturating_add(delegator_reward_trunc);
                                unbond.with_amount(new_amount);
                                provider.write_unbonds(account_hash, unbonds)?;
                                purse
                            }
                            None => {
                                return Err(Error::DelegatorNotFound);
                            }
                        }
                    }
                    Err(err) => return Err(err),
                }
            }
            Err(err) => {
                return Err(err);
            }
        };

        delegator_payouts.push((
            delegator_public_key.to_account_hash(),
            delegator_reward_trunc,
            delegator_bonding_purse,
        ));

        let allocation = SeigniorageAllocation::delegator(
            delegator_public_key,
            validator_public_key.clone(),
            delegator_reward_trunc,
        );

        seigniorage_allocations.push(allocation);
    }

    Ok(delegator_payouts)
}

/// Attempts to apply the validator reward to the existing stake. If the reward recipient has
/// completely unstaked, applies it to their unbond instead. In either case, returns
/// the purse the amount should be applied to.
pub fn distribute_validator_rewards<P>(
    provider: &mut P,
    seigniorage_allocations: &mut Vec<SeigniorageAllocation>,
    validator_public_key: PublicKey,
    amount: U512,
) -> Result<URef, Error>
where
    P: StorageProvider,
{
    let bid_key: Key = BidAddr::from(validator_public_key.clone()).into();
    let bonding_purse = match read_current_validator_bid(provider, bid_key) {
        Ok(mut validator_bid) if !validator_bid.staked_amount().is_zero() => {
            // Only distribute to the bonding purse if the staked amount is not zero - an amount of
            // zero indicates a validator that has unbonded, but whose unbonds haven't been
            // processed yet.
            let purse = *validator_bid.bonding_purse();
            validator_bid.increase_stake(amount)?;
            provider.write_bid(bid_key, BidKind::Validator(validator_bid))?;
            purse
        }
        Ok(_) | Err(Error::ValidatorNotFound) => {
            // check to see if there are unbond entries for this recipient, and if there are
            // apply the amount to the unbond entry with the highest era.
            let account_hash = validator_public_key.to_account_hash();
            match provider.read_unbonds(&account_hash) {
                Ok(mut unbonds) => {
                    match unbonds
                        .iter_mut()
                        .max_by(|x, y| x.era_of_creation().cmp(&y.era_of_creation()))
                    {
                        Some(unbond) => {
                            let purse = *unbond.bonding_purse();
                            let new_amount = unbond.amount().saturating_add(amount);
                            unbond.with_amount(new_amount);
                            provider.write_unbonds(account_hash, unbonds)?;
                            purse
                        }
                        None => {
                            return Err(Error::ValidatorNotFound);
                        }
                    }
                }
                Err(err) => return Err(err),
            }
        }
        Err(err) => return Err(err),
    };

    let allocation = SeigniorageAllocation::validator(validator_public_key, amount);
    seigniorage_allocations.push(allocation);
    Ok(bonding_purse)
}

#[derive(Debug)]
enum UnbondRedelegationOutcome {
    Withdrawal,
    SuccessfullyRedelegated,
    NonexistantRedelegationTarget,
    RedelegationTargetHasNoVacancy,
    RedelegationTargetIsUnstaked,
    DelegationAmountBelowCap,
}

fn handle_redelegation<P>(
    provider: &mut P,
    unbonding_purse: &UnbondingPurse,
    max_delegators_per_validator: u32,
    minimum_delegation_amount: u64,
) -> Result<UnbondRedelegationOutcome, ApiError>
where
    P: StorageProvider + MintProvider + RuntimeProvider,
{
    let redelegation_target_public_key = match unbonding_purse.new_validator() {
        Some(public_key) => {
            // get updated key if `ValidatorBid` public key was changed
            let validator_bid_addr = BidAddr::from(public_key.clone());
            match read_current_validator_bid(provider, validator_bid_addr.into()) {
                Ok(validator_bid) => validator_bid.validator_public_key().clone(),
                Err(_) => return Ok(UnbondRedelegationOutcome::NonexistantRedelegationTarget),
            }
        }
        None => return Ok(UnbondRedelegationOutcome::Withdrawal),
    };

    let redelegation = handle_delegation(
        provider,
        unbonding_purse.unbonder_public_key().clone(),
        redelegation_target_public_key,
        *unbonding_purse.bonding_purse(),
        *unbonding_purse.amount(),
        max_delegators_per_validator,
        minimum_delegation_amount,
    );
    match redelegation {
        Ok(_) => Ok(UnbondRedelegationOutcome::SuccessfullyRedelegated),
        Err(ApiError::AuctionError(err)) if err == Error::BondTooSmall as u8 => {
            Ok(UnbondRedelegationOutcome::RedelegationTargetIsUnstaked)
        }
        Err(ApiError::AuctionError(err)) if err == Error::DelegationAmountTooSmall as u8 => {
            Ok(UnbondRedelegationOutcome::DelegationAmountBelowCap)
        }
        Err(ApiError::AuctionError(err)) if err == Error::ValidatorNotFound as u8 => {
            Ok(UnbondRedelegationOutcome::NonexistantRedelegationTarget)
        }
        Err(ApiError::AuctionError(err)) if err == Error::ExceededDelegatorSizeLimit as u8 => {
            Ok(UnbondRedelegationOutcome::RedelegationTargetHasNoVacancy)
        }
        Err(err) => Err(err),
    }
}

/// If specified validator exists, and if validator is not yet at max delegators count, processes
/// delegation. For a new delegation a delegator bid record will be created to track the delegation,
/// otherwise the existing tracking record will be updated.
#[allow(clippy::too_many_arguments)]
pub fn handle_delegation<P>(
    provider: &mut P,
    delegator_public_key: PublicKey,
    validator_public_key: PublicKey,
    source: URef,
    amount: U512,
    max_delegators_per_validator: u32,
    minimum_delegation_amount: u64,
) -> Result<U512, ApiError>
where
    P: StorageProvider + MintProvider + RuntimeProvider,
{
    if amount.is_zero() {
        return Err(Error::BondTooSmall.into());
    }

    if amount < U512::from(minimum_delegation_amount) {
        return Err(Error::DelegationAmountTooSmall.into());
    }

    let validator_bid_addr = BidAddr::from(validator_public_key.clone());
    // is there such a validator?
    let validator_bid = read_validator_bid(provider, &validator_bid_addr.into())?;

    if validator_bid.staked_amount().is_zero() {
        // The validator has unbonded, but the unbond has not yet been processed, and so an empty
        // bid still exists. Treat this case as if there was no such validator.
        return Err(Error::ValidatorNotFound.into());
    }

    // is there already a record for this delegator?
    let delegator_bid_key =
        BidAddr::new_from_public_keys(&validator_public_key, Some(&delegator_public_key)).into();

    let (target, delegator_bid) = if let Some(BidKind::Delegator(mut delegator_bid)) =
        provider.read_bid(&delegator_bid_key)?
    {
        delegator_bid.increase_stake(amount)?;
        (*delegator_bid.bonding_purse(), delegator_bid)
    } else {
        // is this validator over the delegator limit?
        let delegator_count = provider.delegator_count(&validator_bid_addr)?;
        if delegator_count >= max_delegators_per_validator as usize {
            warn!(
                %delegator_count, %max_delegators_per_validator,
                "delegator_count {}, max_delegators_per_validator {}",
                delegator_count, max_delegators_per_validator
            );
            return Err(Error::ExceededDelegatorSizeLimit.into());
        }

        let bonding_purse = provider.create_purse()?;
        let delegator_bid = Delegator::unlocked(
            delegator_public_key,
            amount,
            bonding_purse,
            validator_public_key,
        );
        (bonding_purse, Box::new(delegator_bid))
    };

    // transfer token to bonding purse
    provider
        .mint_transfer_direct(
            Some(PublicKey::System.to_account_hash()),
            source,
            target,
            amount,
            None,
        )
        .map_err(|_| Error::TransferToDelegatorPurse)?
        .map_err(|mint_error| {
            // Propagate mint contract's error that occured during execution of transfer
            // entrypoint. This will improve UX in case of (for example)
            // unapproved spending limit error.
            ApiError::from(mint_error)
        })?;

    let updated_amount = delegator_bid.staked_amount();
    provider.write_bid(delegator_bid_key, BidKind::Delegator(delegator_bid))?;

    Ok(updated_amount)
}

pub fn read_validator_bid<P>(provider: &mut P, bid_key: &Key) -> Result<Box<ValidatorBid>, Error>
where
    P: StorageProvider + ?Sized,
{
    if !bid_key.is_bid_addr_key() {
        return Err(Error::InvalidKeyVariant);
    }
    if let Some(BidKind::Validator(validator_bid)) = provider.read_bid(bid_key)? {
        Ok(validator_bid)
    } else {
        Err(Error::ValidatorNotFound)
    }
}

/// Returns current `ValidatorBid` in case the public key was changed.
pub fn read_current_validator_bid<P>(
    provider: &mut P,
    mut bid_key: Key,
) -> Result<Box<ValidatorBid>, Error>
where
    P: StorageProvider + ?Sized,
{
    if !bid_key.is_bid_addr_key() {
        return Err(Error::InvalidKeyVariant);
    }

    for _ in 0..MAX_BRIDGE_CHAIN_LENGTH {
        match provider.read_bid(&bid_key)? {
            Some(BidKind::Validator(validator_bid)) => return Ok(validator_bid),
            Some(BidKind::Bridge(bridge)) => {
                let validator_bid_addr = BidAddr::from(bridge.new_validator_public_key().clone());
                bid_key = validator_bid_addr.into();
            }
            _ => break,
        }
    }
    Err(Error::ValidatorNotFound)
}

pub fn read_delegator_bids<P>(
    provider: &mut P,
    validator_public_key: &PublicKey,
) -> Result<Vec<Delegator>, Error>
where
    P: RuntimeProvider + StorageProvider + ?Sized,
{
    let mut ret = vec![];
    let bid_addr = BidAddr::from(validator_public_key.clone());
    let delegator_bid_keys = provider.get_keys_by_prefix(
        &bid_addr
            .delegators_prefix()
            .map_err(|_| Error::Serialization)?,
    )?;
    for delegator_bid_key in delegator_bid_keys {
        let delegator_bid = read_delegator_bid(provider, &delegator_bid_key)?;
        ret.push(*delegator_bid);
    }

    Ok(ret)
}

pub fn read_delegator_bid<P>(provider: &mut P, bid_key: &Key) -> Result<Box<Delegator>, Error>
where
    P: RuntimeProvider + ?Sized + StorageProvider,
{
    if !bid_key.is_bid_addr_key() {
        return Err(Error::InvalidKeyVariant);
    }
    if let Some(BidKind::Delegator(delegator_bid)) = provider.read_bid(bid_key)? {
        Ok(delegator_bid)
    } else {
        Err(Error::DelegatorNotFound)
    }
}

pub fn seigniorage_recipient<P>(
    provider: &mut P,
    validator_bid: &ValidatorBid,
) -> Result<SeigniorageRecipient, Error>
where
    P: RuntimeProvider + ?Sized + StorageProvider,
{
    let mut delegator_stake: BTreeMap<PublicKey, U512> = BTreeMap::new();
    for delegator_bid in read_delegator_bids(provider, validator_bid.validator_public_key())? {
        if delegator_bid.staked_amount().is_zero() {
            continue;
        }
        delegator_stake.insert(
            delegator_bid.delegator_public_key().clone(),
            delegator_bid.staked_amount(),
        );
    }
    Ok(SeigniorageRecipient::new(
        validator_bid.staked_amount(),
        *validator_bid.delegation_rate(),
        delegator_stake,
    ))
}

/// Returns the era validators from a snapshot.
///
/// This is `pub` as it is used not just in the relevant auction entry point, but also by the
/// engine state while directly querying for the era validators.
pub fn era_validators_from_snapshot(snapshot: SeigniorageRecipientsSnapshot) -> EraValidators {
    snapshot
        .into_iter()
        .map(|(era_id, recipients)| {
            let validator_weights = recipients
                .into_iter()
                .filter_map(|(public_key, bid)| bid.total_stake().map(|stake| (public_key, stake)))
                .collect::<ValidatorWeights>();
            (era_id, validator_weights)
        })
        .collect()
}

/// Initializes the vesting schedule of provided bid if the provided timestamp is greater than
/// or equal to the bid's initial release timestamp and the bid is owned by a genesis
/// validator.
///
/// Returns `true` if the provided bid's vesting schedule was initialized.
pub fn process_with_vesting_schedule<P>(
    provider: &mut P,
    validator_bid: &mut ValidatorBid,
    timestamp_millis: u64,
    vesting_schedule_period_millis: u64,
) -> Result<bool, Error>
where
    P: StorageProvider + RuntimeProvider + ?Sized,
{
    let validator_public_key = validator_bid.validator_public_key().clone();

    let delegator_bids = read_delegator_bids(provider, &validator_public_key)?;
    for mut delegator_bid in delegator_bids {
        let delegator_staked_amount = delegator_bid.staked_amount();
        let delegator_vesting_schedule = match delegator_bid.vesting_schedule_mut() {
            Some(vesting_schedule) => vesting_schedule,
            None => continue,
        };
        if timestamp_millis < delegator_vesting_schedule.initial_release_timestamp_millis() {
            continue;
        }
        if delegator_vesting_schedule
            .initialize_with_schedule(delegator_staked_amount, vesting_schedule_period_millis)
        {
            let delegator_bid_addr = BidAddr::new_from_public_keys(
                &validator_public_key,
                Some(delegator_bid.delegator_public_key()),
            );
            provider.write_bid(
                delegator_bid_addr.into(),
                BidKind::Delegator(Box::new(delegator_bid)),
            )?;
        }
    }

    let validator_staked_amount = validator_bid.staked_amount();
    let validator_vesting_schedule = match validator_bid.vesting_schedule_mut() {
        Some(vesting_schedule) => vesting_schedule,
        None => return Ok(false),
    };
    if timestamp_millis < validator_vesting_schedule.initial_release_timestamp_millis() {
        Ok(false)
    } else {
        Ok(validator_vesting_schedule
            .initialize_with_schedule(validator_staked_amount, vesting_schedule_period_millis))
    }
}

/// Returns the total staked amount of validator + all delegators
pub fn total_staked_amount<P>(provider: &mut P, validator_bid: &ValidatorBid) -> Result<U512, Error>
where
    P: RuntimeProvider + ?Sized + StorageProvider,
{
    let bid_addr = BidAddr::from(validator_bid.validator_public_key().clone());
    let delegator_bid_keys = provider.get_keys_by_prefix(
        &bid_addr
            .delegators_prefix()
            .map_err(|_| Error::Serialization)?,
    )?;

    let mut sum = U512::zero();

    for delegator_bid_key in delegator_bid_keys {
        let delegator = read_delegator_bid(provider, &delegator_bid_key)?;
        let staked_amount = delegator.staked_amount();
        sum += staked_amount;
    }

    sum.checked_add(validator_bid.staked_amount())
        .ok_or(Error::InvalidAmount)
}

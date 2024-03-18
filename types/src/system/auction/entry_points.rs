use crate::{
    system::auction::{
        DelegationRate, ValidatorWeights, ARG_AMOUNT, ARG_DELEGATION_RATE, ARG_DELEGATOR,
        ARG_ERA_END_TIMESTAMP_MILLIS, ARG_NEW_VALIDATOR, ARG_PUBLIC_KEY, ARG_VALIDATOR,
        ARG_VALIDATOR_PUBLIC_KEY, METHOD_ACTIVATE_BID, METHOD_ADD_BID, METHOD_DELEGATE,
        METHOD_DISTRIBUTE, METHOD_GET_ERA_VALIDATORS, METHOD_READ_ERA_ID, METHOD_REDELEGATE,
        METHOD_RUN_AUCTION, METHOD_SLASH, METHOD_UNDELEGATE, METHOD_WITHDRAW_BID,
    },
    CLType, CLTyped, EntryPoint, EntryPointAccess, EntryPointType, EntryPoints, Parameter,
    PublicKey, U512,
};

use super::{
    ARG_MAXIMUM_DELEGATION_AMOUNT, ARG_MINIMUM_DELEGATION_AMOUNT, ARG_REWARDS_MAP,
    METHOD_FORCED_UNDELEGATE,
};

/// Creates auction contract entry points.
pub fn auction_entry_points() -> EntryPoints {
    let mut entry_points = EntryPoints::new();

    let entry_point = EntryPoint::new(
        METHOD_GET_ERA_VALIDATORS,
        vec![],
        Option::<ValidatorWeights>::cl_type(),
        EntryPointAccess::Public,
        EntryPointType::AddressableEntity,
    );
    entry_points.add_entry_point(entry_point);

    let entry_point = EntryPoint::new(
        METHOD_ADD_BID,
        vec![
            Parameter::new(ARG_PUBLIC_KEY, PublicKey::cl_type()),
            Parameter::new(ARG_DELEGATION_RATE, DelegationRate::cl_type()),
            Parameter::new(ARG_AMOUNT, U512::cl_type()),
            Parameter::new(ARG_MINIMUM_DELEGATION_AMOUNT, u64::cl_type()),
            Parameter::new(ARG_MAXIMUM_DELEGATION_AMOUNT, u64::cl_type()),
        ],
        U512::cl_type(),
        EntryPointAccess::Public,
        EntryPointType::AddressableEntity,
    );
    entry_points.add_entry_point(entry_point);

    let entry_point = EntryPoint::new(
        METHOD_WITHDRAW_BID,
        vec![
            Parameter::new(ARG_PUBLIC_KEY, PublicKey::cl_type()),
            Parameter::new(ARG_AMOUNT, U512::cl_type()),
        ],
        U512::cl_type(),
        EntryPointAccess::Public,
        EntryPointType::AddressableEntity,
    );
    entry_points.add_entry_point(entry_point);

    let entry_point = EntryPoint::new(
        METHOD_DELEGATE,
        vec![
            Parameter::new(ARG_DELEGATOR, PublicKey::cl_type()),
            Parameter::new(ARG_VALIDATOR, PublicKey::cl_type()),
            Parameter::new(ARG_AMOUNT, U512::cl_type()),
        ],
        U512::cl_type(),
        EntryPointAccess::Public,
        EntryPointType::AddressableEntity,
    );
    entry_points.add_entry_point(entry_point);

    let entry_point = EntryPoint::new(
        METHOD_UNDELEGATE,
        vec![
            Parameter::new(ARG_DELEGATOR, PublicKey::cl_type()),
            Parameter::new(ARG_VALIDATOR, PublicKey::cl_type()),
            Parameter::new(ARG_AMOUNT, U512::cl_type()),
        ],
        U512::cl_type(),
        EntryPointAccess::Public,
        EntryPointType::AddressableEntity,
    );
    entry_points.add_entry_point(entry_point);

    let entry_point = EntryPoint::new(
        METHOD_REDELEGATE,
        vec![
            Parameter::new(ARG_DELEGATOR, PublicKey::cl_type()),
            Parameter::new(ARG_VALIDATOR, PublicKey::cl_type()),
            Parameter::new(ARG_AMOUNT, U512::cl_type()),
            Parameter::new(ARG_NEW_VALIDATOR, PublicKey::cl_type()),
        ],
        U512::cl_type(),
        EntryPointAccess::Public,
        EntryPointType::AddressableEntity,
    );
    entry_points.add_entry_point(entry_point);

    let entry_point = EntryPoint::new(
        METHOD_FORCED_UNDELEGATE,
        vec![],
        CLType::Unit,
        EntryPointAccess::Public,
        EntryPointType::AddressableEntity,
    );
    entry_points.add_entry_point(entry_point);

    let entry_point = EntryPoint::new(
        METHOD_RUN_AUCTION,
        vec![Parameter::new(ARG_ERA_END_TIMESTAMP_MILLIS, u64::cl_type())],
        CLType::Unit,
        EntryPointAccess::Public,
        EntryPointType::AddressableEntity,
    );
    entry_points.add_entry_point(entry_point);

    let entry_point = EntryPoint::new(
        METHOD_SLASH,
        vec![],
        CLType::Unit,
        EntryPointAccess::Public,
        EntryPointType::AddressableEntity,
    );
    entry_points.add_entry_point(entry_point);

    let entry_point = EntryPoint::new(
        METHOD_DISTRIBUTE,
        vec![Parameter::new(
            ARG_REWARDS_MAP,
            CLType::map(CLType::PublicKey, CLType::U512),
        )],
        CLType::Unit,
        EntryPointAccess::Public,
        EntryPointType::AddressableEntity,
    );
    entry_points.add_entry_point(entry_point);

    let entry_point = EntryPoint::new(
        METHOD_READ_ERA_ID,
        vec![],
        CLType::U64,
        EntryPointAccess::Public,
        EntryPointType::AddressableEntity,
    );
    entry_points.add_entry_point(entry_point);

    let entry_point = EntryPoint::new(
        METHOD_ACTIVATE_BID,
        vec![Parameter::new(ARG_VALIDATOR_PUBLIC_KEY, CLType::PublicKey)],
        CLType::Unit,
        EntryPointAccess::Public,
        EntryPointType::AddressableEntity,
    );
    entry_points.add_entry_point(entry_point);

    entry_points
}

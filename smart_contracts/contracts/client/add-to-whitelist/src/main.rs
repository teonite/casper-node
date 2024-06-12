#![no_std]
#![no_main]

extern crate alloc;

use casper_contract::contract_api::{runtime, system};
use casper_types::{
    runtime_args,
    system::auction,
    PublicKey,
};

fn add_to_whitelist(validator: PublicKey, delegator: PublicKey) {
    // TODO(jck): auction contract?
    let contract_hash = system::get_auction();
    let args = runtime_args! {
        auction::ARG_VALIDATOR => validator,
        auction::ARG_DELEGATOR => delegator,
    };
    runtime::call_contract::<()>(contract_hash, auction::METHOD_ADD_TO_WHITELIST, args);
}

// TODO(jck): rewrite docstring
// Change validator bid public key.
//
// Accepts current bid's public key and new public key.
// Updates existing validator bid and all related delegator bids with
// the new public key.
#[no_mangle]
pub extern "C" fn call() {
    let validator = runtime::get_named_arg(auction::ARG_VALIDATOR);
    let delegator = runtime::get_named_arg(auction::ARG_DELEGATOR);
    add_to_whitelist(validator, delegator);
}

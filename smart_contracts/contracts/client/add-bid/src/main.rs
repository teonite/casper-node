#![no_std]
#![no_main]

extern crate alloc;

use casper_contract::contract_api::{runtime, system};
use casper_types::{
    runtime_args,
    system::auction::{self, DelegationRate, WhitelistSize},
    PublicKey, U512,
};

const ARG_AMOUNT: &str = "amount";
const ARG_DELEGATION_RATE: &str = "delegation_rate";
const ARG_WHITELIST_SIZE: &str = "whitelist_size";
const ARG_PUBLIC_KEY: &str = "public_key";

fn add_bid(
    public_key: PublicKey,
    bond_amount: U512,
    delegation_rate: DelegationRate,
    whitelist_size: WhitelistSize,
) {
    let contract_hash = system::get_auction();
    let args = runtime_args! {
        auction::ARG_PUBLIC_KEY => public_key,
        auction::ARG_AMOUNT => bond_amount,
        auction::ARG_DELEGATION_RATE => delegation_rate,
        auction::ARG_WHITELIST_SIZE => whitelist_size,
    };
    runtime::call_contract::<U512>(contract_hash, auction::METHOD_ADD_BID, args);
}

// Bidding contract.
//
// Accepts a public key, amount, delegation rate and whitelist size.
// Issues an add bid request to the auction contract.
#[no_mangle]
pub extern "C" fn call() {
    let public_key = runtime::get_named_arg(ARG_PUBLIC_KEY);
    let bond_amount = runtime::get_named_arg(ARG_AMOUNT);
    let delegation_rate = runtime::get_named_arg(ARG_DELEGATION_RATE);
    let whitelist_size = runtime::get_named_arg(ARG_WHITELIST_SIZE);

    add_bid(public_key, bond_amount, delegation_rate, whitelist_size);
}

use cosmwasm_std::Uint128;
use cw_storage_plus::Item;

pub const AMOUNT: Item<Uint128> = Item::new("amount");
pub const DENOM: Item<String> = Item::new("denom");

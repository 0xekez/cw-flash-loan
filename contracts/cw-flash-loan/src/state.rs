use cosmwasm_std::{Addr, Decimal, Uint128};
use cw_storage_plus::{Item, Map};

pub const ADMIN: Item<Option<Addr>> = Item::new("admin");
pub const FEE: Item<Decimal> = Item::new("fee");
pub const LOAN_DENOM: Item<String> = Item::new("loan_denom");

/// Map between addresses and the amount they have provided.
pub const PROVISIONS: Map<Addr, Uint128> = Map::new("provision");
pub const TOTAL_PROVIDED: Item<Uint128> = Item::new("total_provided");

use cosmwasm_std::IbcMsg;
use cw_storage_plus::Item;

pub const MSG: Item<IbcMsg> = Item::new("msg");

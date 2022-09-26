use cosmwasm_schema::cw_serde;
use cosmwasm_std::Uint128;

#[cw_serde]
pub struct InstantiateMsg {
    pub amount: Uint128,
    pub denom: String,
}

#[cw_serde]
pub enum ExecuteMsg {
    Receive(cw20::Cw20ReceiveMsg),
    ReceiveLoan {},
    Update { amount: Uint128, denom: String },
}

#[cw_serde]
pub enum QueryMsg {}

use cosmwasm_std::StdError;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ContractError {
    #[error("{0}")]
    Std(#[from] StdError),

    #[error("Unauthorized")]
    Unauthorized {},

    #[error("Loan did not match configured denom ({expected})")]
    Denom { expected: String },

    #[error("Invalid funds. Expected denom ({denom})")]
    WrongFunds { denom: String },

    #[error("Attempted to provide native tokens when cw20 tokens were expected")]
    Cw20Expected {},

    #[error("Attempted to provide cw20 tokens when native tokens were expected")]
    NativeExpected {},

    #[error("Funds + fee was not returned")]
    NotReturned {},

    #[error("Can not withdraw without providing first")]
    NoProvisions {},
}

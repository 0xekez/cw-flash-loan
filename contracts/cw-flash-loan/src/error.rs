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

    #[error("Funds + fee was not returned")]
    NotReturned {},
}

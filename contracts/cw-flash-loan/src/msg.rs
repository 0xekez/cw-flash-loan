use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::{Decimal, Deps, StdResult, Uint128};

use crate::state::CheckedLoanDenom;

#[cw_serde]
pub enum LoanDenom {
    Cw20 { address: String },
    Native { denom: String },
}

#[cw_serde]
pub struct InstantiateMsg {
    pub admin: Option<String>,
    pub fee: Decimal,
    pub loan_denom: LoanDenom,
}

#[cw_serde]
pub enum ExecuteMsg {
    UpdateConfig { admin: Option<String>, fee: Decimal },
    Loan { receiver: String, amount: Uint128 },
    AssertBalance { amount: Uint128 },
    Provide {},
    Withdraw {},
    Receive(cw20::Cw20ReceiveMsg),
}

#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    #[returns(ConfigResponse)]
    GetConfig {},
    #[returns(Uint128)]
    Provided { address: String },
    #[returns(Uint128)]
    TotalProvided {},
    #[returns(Uint128)]
    Entitled { address: String },
    #[returns(Uint128)]
    Balance {},
}

#[cw_serde]
pub struct ConfigResponse {
    pub admin: Option<String>,
    pub fee: Decimal,
    pub loan_denom: CheckedLoanDenom,
}

#[cw_serde]
pub enum LoanMsg {
    ReceiveLoan {},
}

impl LoanDenom {
    pub fn into_checked(self, deps: Deps) -> StdResult<CheckedLoanDenom> {
        Ok(match self {
            LoanDenom::Cw20 { address } => {
                let address = deps.api.addr_validate(&address)?;
                CheckedLoanDenom::Cw20 { address }
            }
            LoanDenom::Native { denom } => CheckedLoanDenom::Native { denom },
        })
    }
}

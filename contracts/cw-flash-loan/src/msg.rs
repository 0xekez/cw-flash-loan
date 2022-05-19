use cosmwasm_std::{Decimal, Deps, StdResult, Uint128};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::state::CheckedLoanDenom;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub enum LoanDenom {
    Cw20 { address: String },
    Native { denom: String },
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct InstantiateMsg {
    pub admin: Option<String>,
    pub fee: Decimal,
    pub loan_denom: LoanDenom,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ExecuteMsg {
    UpdateConfig { admin: Option<String>, fee: Decimal },
    Loan { receiver: String, amount: Uint128 },
    AssertBalance { amount: Uint128 },
    Provide {},
    Withdraw {},
    ReceiveCw20(cw20::Cw20ReceiveMsg),
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum QueryMsg {
    GetConfig {},
    Provided { address: String },
    TotalProvided {},
    Entitled { address: String },
    Balance {},
}

// We define a custom struct for each query response
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct ConfigResponse {
    pub admin: Option<String>,
    pub fee: Decimal,
    pub loan_denom: CheckedLoanDenom,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
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

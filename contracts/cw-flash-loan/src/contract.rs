use std::convert::TryInto;

#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    to_binary, Addr, BankMsg, Binary, Coin, CosmosMsg, Decimal, Deps, DepsMut, Env, MessageInfo,
    Response, StdResult, Uint128, Uint256, WasmMsg,
};
use cw2::set_contract_version;

use crate::error::ContractError;
use crate::msg::{ConfigResponse, ExecuteMsg, InstantiateMsg, LoanMsg, QueryMsg};
use crate::state::{CheckedLoanDenom, ADMIN, FEE, LOAN_DENOM, PROVISIONS, TOTAL_PROVIDED};

// version info for migration info
const CONTRACT_NAME: &str = "crates.io:cw-flash-loan";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

    let admin = msg.admin.map(|a| deps.api.addr_validate(&a)).transpose()?;
    let loan_denom = msg.loan_denom.into_checked(deps.as_ref())?;

    ADMIN.save(deps.storage, &admin)?;
    FEE.save(deps.storage, &msg.fee)?;
    LOAN_DENOM.save(deps.storage, &loan_denom)?;
    TOTAL_PROVIDED.save(deps.storage, &Uint128::zero())?;

    Ok(Response::new()
        .add_attribute("method", "instantiate")
        .add_attribute(
            "admin",
            admin
                .map(|a| a.to_string())
                .unwrap_or_else(|| "None".to_string()),
        )
        .add_attribute("fee", msg.fee.to_string()))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    match msg {
        ExecuteMsg::UpdateConfig { admin, fee } => {
            execute_update_config(deps, info.sender, admin, fee)
        }
        ExecuteMsg::Loan { receiver, amount } => execute_loan(deps, env, receiver, amount),
        ExecuteMsg::AssertBalance { amount } => execute_assert_balance(deps.as_ref(), env, amount),
        ExecuteMsg::Provide {} => execute_provide_native(deps, env, info),
        ExecuteMsg::Withdraw {} => execute_withdraw(deps, env, info),
        ExecuteMsg::Receive(cw20::Cw20ReceiveMsg {
            sender,
            amount,
            // Intentionally ignore message field. No additional
            // validation really to be done with this.
            msg: _,
        }) => execute_provide_cw20(deps, env, info, sender, amount),
    }
}

pub fn execute_update_config(
    deps: DepsMut,
    sender: Addr,
    new_admin: Option<String>,
    new_fee: Decimal,
) -> Result<Response, ContractError> {
    let admin = ADMIN.load(deps.storage)?;
    if Some(sender) != admin {
        return Err(ContractError::Unauthorized {});
    }

    let new_admin = new_admin.map(|a| deps.api.addr_validate(&a)).transpose()?;
    ADMIN.save(deps.storage, &new_admin)?;

    FEE.save(deps.storage, &new_fee)?;

    Ok(Response::new()
        .add_attribute("method", "update_config")
        .add_attribute(
            "new_admin",
            new_admin
                .map(|a| a.to_string())
                .unwrap_or_else(|| "None".to_string()),
        )
        .add_attribute("new_fee", new_fee.to_string()))
}

pub fn execute_loan(
    deps: DepsMut,
    env: Env,
    receiver: String,
    amount: Uint128,
) -> Result<Response, ContractError> {
    let fee = FEE.load(deps.storage)?;
    let loan_denom = LOAN_DENOM.load(deps.storage)?;

    let avaliable = query_avaliable_balance(deps.as_ref(), &env, &loan_denom)?;

    let execute_msg = match loan_denom {
        CheckedLoanDenom::Cw20 { address } => WasmMsg::Execute {
            contract_addr: address.into_string(),
            msg: to_binary(&cw20::Cw20ExecuteMsg::Send {
                amount,
                msg: to_binary(&LoanMsg::ReceiveLoan {})?,
                contract: receiver.clone(),
            })?,
            funds: vec![],
        },
        CheckedLoanDenom::Native { denom } => WasmMsg::Execute {
            contract_addr: receiver.clone(),
            msg: to_binary(&LoanMsg::ReceiveLoan {})?,
            funds: vec![Coin { amount, denom }],
        },
    };

    // Expect that we will get everything back plus the fee applied to
    // the amount borrowed. For example, if the contract holds 200
    // tokens and the fee is 0.03 a loan for 100 tokens should result
    // in 203 tokens being held by the contract.
    let expected = avaliable + (fee * amount);

    let return_msg = WasmMsg::Execute {
        contract_addr: env.contract.address.into_string(),
        msg: to_binary(&ExecuteMsg::AssertBalance { amount: expected })?,
        funds: vec![],
    };

    Ok(Response::new()
        .add_attribute("method", "loan")
        .add_attribute("receiver", receiver)
        .add_message(execute_msg)
        .add_message(return_msg))
}

fn get_only_denom_amount(funds: Vec<Coin>, denom: String) -> Result<Uint128, ContractError> {
    if funds.len() != 1 {
        return Err(ContractError::WrongFunds { denom });
    }
    let provided = funds.into_iter().next().unwrap();
    if provided.denom != denom {
        return Err(ContractError::WrongFunds { denom });
    }
    Ok(provided.amount)
}

pub fn execute_provide_cw20(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    sender: String,
    amount: Uint128,
) -> Result<Response, ContractError> {
    let loan_denom = LOAN_DENOM.load(deps.storage)?;
    let loan_token = match &loan_denom {
        CheckedLoanDenom::Cw20 { address } => address,
        CheckedLoanDenom::Native { .. } => return Err(ContractError::NativeExpected {}),
    };
    if *loan_token != info.sender {
        return Err(ContractError::Unauthorized {});
    }
    let sender = deps.api.addr_validate(&sender)?;

    let provided = amount;
    let balance = query_avaliable_balance(deps.as_ref(), &env, &loan_denom)? - provided;
    let total_provied = TOTAL_PROVIDED.load(deps.storage)?;

    let amount_to_provide = if total_provied.is_zero() || balance.is_zero() {
        provided
    } else {
        // The amount you receive is (balance per provided) *
        // provided. This stops being able to withdraw and then
        // instantly deposit to drain the rewards from the contract.
        (total_provied.full_mul(provided) / Uint256::from_uint128(balance))
            .try_into()
            .unwrap()
    };

    PROVISIONS.update(deps.storage, sender.clone(), |old| -> StdResult<_> {
        Ok(old.unwrap_or_default().checked_add(amount_to_provide)?)
    })?;
    TOTAL_PROVIDED.update(deps.storage, |old| -> StdResult<_> {
        Ok(old.checked_add(amount_to_provide)?)
    })?;

    Ok(Response::new()
        .add_attribute("method", "provide_native")
        .add_attribute("provider", sender)
        .add_attribute("provided", amount_to_provide))
}

pub fn execute_provide_native(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
) -> Result<Response, ContractError> {
    let MessageInfo { sender, funds } = info;
    let loan_denom = LOAN_DENOM.load(deps.storage)?;
    let provided = match &loan_denom {
        CheckedLoanDenom::Cw20 { .. } => return Err(ContractError::Cw20Expected {}),
        CheckedLoanDenom::Native { denom } => get_only_denom_amount(funds, denom.clone())?,
    };

    // Need to use balance before we provided for this computaton as
    // our porportional entitlement needs to be set at the old rate.
    let balance = query_avaliable_balance(deps.as_ref(), &env, &loan_denom)? - provided;
    let total_provied = TOTAL_PROVIDED.load(deps.storage)?;

    let amount_to_provide = if total_provied.is_zero() || balance.is_zero() {
        provided
    } else {
        // The amount you receive is (balance per provided) *
        // provided. This stops being able to withdraw and then
        // instantly deposit to drain the rewards from the contract.
        (total_provied.full_mul(provided) / Uint256::from_uint128(balance))
            .try_into()
            .unwrap()
    };

    PROVISIONS.update(deps.storage, sender.clone(), |old| -> StdResult<_> {
        Ok(old.unwrap_or_default().checked_add(amount_to_provide)?)
    })?;
    TOTAL_PROVIDED.update(deps.storage, |old| -> StdResult<_> {
        Ok(old.checked_add(amount_to_provide)?)
    })?;

    Ok(Response::new()
        .add_attribute("method", "provide_native")
        .add_attribute("provider", sender)
        .add_attribute("provided", amount_to_provide))
}

fn compute_entitled(provided: Uint128, total_provided: Uint128, avaliable: Uint128) -> Uint128 {
    (avaliable.full_mul(provided) / Uint256::from_uint128(total_provided))
        .try_into()
        .unwrap()
}

pub fn execute_withdraw(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
) -> Result<Response, ContractError> {
    let MessageInfo { sender, .. } = info;
    let loan_denom = LOAN_DENOM.load(deps.storage)?;
    let total_provided = TOTAL_PROVIDED.load(deps.storage)?;

    let provided = if let Some(provision) = PROVISIONS.may_load(deps.storage, sender.clone())? {
        Ok(provision)
    } else {
        Err(ContractError::NoProvisions {})
    }?;

    let avaliable = query_avaliable_balance(deps.as_ref(), &env, &loan_denom)?;

    let entitled = compute_entitled(provided, total_provided, avaliable);

    PROVISIONS.save(deps.storage, sender.clone(), &Uint128::zero())?;
    TOTAL_PROVIDED.update(deps.storage, |old| -> StdResult<_> {
        Ok(old.checked_sub(provided)?)
    })?;

    let withdraw_message: CosmosMsg = match loan_denom {
        CheckedLoanDenom::Cw20 { address } => WasmMsg::Execute {
            contract_addr: address.into_string(),
            msg: to_binary(&cw20::Cw20ExecuteMsg::Transfer {
                recipient: sender.to_string(),
                amount: entitled,
            })?,
            funds: vec![],
        }
        .into(),
        CheckedLoanDenom::Native { denom } => BankMsg::Send {
            to_address: sender.to_string(),
            amount: vec![Coin {
                amount: entitled,
                denom,
            }],
        }
        .into(),
    };

    Ok(Response::new()
        .add_attribute("method", "withdraw")
        .add_attribute("receiver", sender)
        .add_attribute("amount", entitled)
        .add_message(withdraw_message))
}

pub fn execute_assert_balance(
    deps: Deps,
    env: Env,
    expected: Uint128,
) -> Result<Response, ContractError> {
    let loan_denom = LOAN_DENOM.load(deps.storage)?;

    let avaliable = query_avaliable_balance(deps, &env, &loan_denom)?;

    if avaliable != expected {
        Err(ContractError::NotReturned {})
    } else {
        Ok(Response::new().add_attribute("method", "assert_balances"))
    }
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::GetConfig {} => query_get_config(deps),
        QueryMsg::Provided { address } => query_provided(deps, address),
        QueryMsg::TotalProvided {} => query_total_provided(deps),
        QueryMsg::Entitled { address } => query_entitled(deps, env, address),
        QueryMsg::Balance {} => query_balance(deps, env),
    }
}

fn query_avaliable_balance(
    deps: Deps,
    env: &Env,
    loan_denom: &CheckedLoanDenom,
) -> StdResult<Uint128> {
    Ok(match loan_denom {
        CheckedLoanDenom::Cw20 { address } => {
            let balance: cw20::BalanceResponse = deps.querier.query_wasm_smart(
                address.to_string(),
                &cw20::Cw20QueryMsg::Balance {
                    address: env.contract.address.to_string(),
                },
            )?;
            balance.balance
        }
        CheckedLoanDenom::Native { denom } => {
            deps.querier
                .query_balance(&env.contract.address, denom)?
                .amount
        }
    })
}

pub fn query_get_config(deps: Deps) -> StdResult<Binary> {
    let admin = ADMIN.load(deps.storage)?;
    let fee = FEE.load(deps.storage)?;
    let loan_denom = LOAN_DENOM.load(deps.storage)?;

    to_binary(&ConfigResponse {
        admin: admin.map(|a| a.into()),
        fee,
        loan_denom,
    })
}

pub fn query_provided(deps: Deps, address: String) -> StdResult<Binary> {
    let address = deps.api.addr_validate(&address)?;
    let provided = PROVISIONS
        .may_load(deps.storage, address)
        .unwrap_or_default();

    match provided {
        Some(provided) => to_binary(&provided),
        None => to_binary(&Uint128::zero()),
    }
}

pub fn query_total_provided(deps: Deps) -> StdResult<Binary> {
    let total = TOTAL_PROVIDED.load(deps.storage)?;
    to_binary(&total)
}

pub fn query_entitled(deps: Deps, env: Env, address: String) -> StdResult<Binary> {
    let address = deps.api.addr_validate(&address)?;

    let loan_denom = LOAN_DENOM.load(deps.storage)?;
    let provided = PROVISIONS.may_load(deps.storage, address)?;

    match provided {
        Some(provided) => {
            let total_provided = TOTAL_PROVIDED.load(deps.storage)?;

            let avaliable = query_avaliable_balance(deps, &env, &loan_denom)?;

            let entitled = compute_entitled(provided, total_provided, avaliable);

            to_binary(&entitled)
        }
        None => to_binary(&Uint128::zero()),
    }
}

pub fn query_balance(deps: Deps, env: Env) -> StdResult<Binary> {
    let loan_denom = LOAN_DENOM.load(deps.storage)?;
    let avaliable = query_avaliable_balance(deps, &env, &loan_denom)?;

    to_binary(&avaliable)
}

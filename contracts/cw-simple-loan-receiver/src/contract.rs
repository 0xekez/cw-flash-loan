#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    to_binary, Addr, BankMsg, Binary, Coin, Deps, DepsMut, Env, MessageInfo, Response, StdResult,
    Uint128, WasmMsg,
};

use crate::error::ContractError;
use crate::msg::{ExecuteMsg, InstantiateMsg, QueryMsg};
use crate::state::{AMOUNT, DENOM};

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    AMOUNT.save(deps.storage, &msg.amount)?;
    DENOM.save(deps.storage, &msg.denom)?;
    Ok(Response::new().add_attribute("method", "instantiate"))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    eprintln!("hmm");
    match msg {
        ExecuteMsg::ReceiveLoan {} => execute_receive_loan(info.sender, deps.as_ref()),
        ExecuteMsg::Update { amount, denom } => execute_update(deps, amount, denom),
        ExecuteMsg::Receive(cw20::Cw20ReceiveMsg { sender, .. }) => {
            execute_receive_cw20_loan(info.sender, sender, deps.as_ref())
        }
    }
}

pub fn execute_update(
    deps: DepsMut,
    amount: Uint128,
    denom: String,
) -> Result<Response, ContractError> {
    AMOUNT.save(deps.storage, &amount)?;
    DENOM.save(deps.storage, &denom)?;
    Ok(Response::new().add_attribute("method", "execute_update_fee"))
}

pub fn execute_receive_cw20_loan(
    token_address: Addr,
    sender: String,
    deps: Deps,
) -> Result<Response, ContractError> {
    let amount = AMOUNT.load(deps.storage)?;
    let msg = cw20::Cw20ExecuteMsg::Transfer {
        recipient: sender,
        amount,
    };
    let msg = WasmMsg::Execute {
        contract_addr: token_address.into_string(),
        msg: to_binary(&msg)?,
        funds: vec![],
    };

    Ok(Response::new().add_message(msg))
}

pub fn execute_receive_loan(sender: Addr, deps: Deps) -> Result<Response, ContractError> {
    let amount = AMOUNT.load(deps.storage)?;
    let denom = DENOM.load(deps.storage)?;

    let msg = BankMsg::Send {
        to_address: sender.into_string(),
        amount: vec![Coin { amount, denom }],
    };

    Ok(Response::new().add_message(msg))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(_deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {}
}

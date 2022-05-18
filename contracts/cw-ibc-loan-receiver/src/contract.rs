#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{Binary, Deps, DepsMut, Env, IbcMsg, MessageInfo, Response, StdResult};

use crate::error::ContractError;
use crate::msg::{ExecuteMsg, InstantiateMsg, QueryMsg};
use crate::state::MSG;

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    MSG.save(deps.storage, &msg.msg)?;
    Ok(Response::default())
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    match msg {
        ExecuteMsg::Receive {} => execute_receive(deps.as_ref()),
        ExecuteMsg::Update { msg } => execute_update(deps, msg),
    }
}

pub fn execute_update(deps: DepsMut, msg: IbcMsg) -> Result<Response, ContractError> {
    MSG.save(deps.storage, &msg)?;
    Ok(Response::new().add_attribute("method", "execute_update"))
}

pub fn execute_receive(deps: Deps) -> Result<Response, ContractError> {
    let msg = MSG.load(deps.storage)?;

    Ok(Response::new().add_message(msg))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(_deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {}
}

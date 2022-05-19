use cosmwasm_std::{Addr, Decimal, Empty, Uint128};
use cw20::Cw20Coin;
use cw_multi_test::{App, Contract, ContractWrapper, Executor};

use crate::{
    msg::{ExecuteMsg, InstantiateMsg, LoanDenom},
    ContractError,
};

const CREATOR_ADDR: &str = "creator";

fn flash_loan_contract() -> Box<dyn Contract<Empty>> {
    let contract = ContractWrapper::new(
        crate::contract::execute,
        crate::contract::instantiate,
        crate::contract::query,
    );
    Box::new(contract)
}

fn simple_receiver_contract() -> Box<dyn Contract<Empty>> {
    let contract = ContractWrapper::new(
        cw_simple_loan_receiver::contract::execute,
        cw_simple_loan_receiver::contract::instantiate,
        cw_simple_loan_receiver::contract::query,
    );
    Box::new(contract)
}

fn cw20_contract() -> Box<dyn Contract<Empty>> {
    let contract = ContractWrapper::new(
        cw20_base::contract::execute,
        cw20_base::contract::instantiate,
        cw20_base::contract::query,
    );
    Box::new(contract)
}

struct SetupTestResponse {
    app: App,
    flash_loan: Addr,
    receiver: Addr,
}

fn setup_test(
    flash_balance: Uint128,
    receiver_balance: Uint128,
    receiver_return_amount: Uint128,
    fee: Decimal,
) -> SetupTestResponse {
    let mut app = App::default();

    let flash_code = app.store_code(flash_loan_contract());
    let receiver_code = app.store_code(simple_receiver_contract());
    let cw20_code = app.store_code(cw20_contract());

    let receiver = app
        .instantiate_contract(
            receiver_code,
            Addr::unchecked(CREATOR_ADDR),
            &cw_simple_loan_receiver::msg::InstantiateMsg {
                amount: receiver_return_amount,
                denom: "ujuno".to_string(),
            },
            &[],
            "receiver",
            None,
        )
        .unwrap();

    let cw20 = app
        .instantiate_contract(
            cw20_code,
            Addr::unchecked(CREATOR_ADDR),
            &cw20_base::msg::InstantiateMsg {
                name: "Floob Token".to_string(),
                symbol: "FLOOB".to_string(),
                decimals: 6,
                initial_balances: vec![
                    Cw20Coin {
                        address: CREATOR_ADDR.to_string(),
                        amount: flash_balance,
                    },
                    Cw20Coin {
                        address: receiver.to_string(),
                        amount: receiver_balance,
                    },
                ],
                mint: None,
                marketing: None,
            },
            &[],
            "floob token",
            None,
        )
        .unwrap();

    let flash_loan = app
        .instantiate_contract(
            flash_code,
            Addr::unchecked(CREATOR_ADDR),
            &InstantiateMsg {
                admin: Some(CREATOR_ADDR.to_string()),
                fee,
                loan_denom: LoanDenom::Cw20 {
                    address: cw20.to_string(),
                },
            },
            &[],
            "flash-loan",
            None,
        )
        .unwrap();

    app.execute_contract(
        Addr::unchecked(CREATOR_ADDR),
        cw20,
        &cw20::Cw20ExecuteMsg::Transfer {
            recipient: flash_loan.to_string(),
            amount: Uint128::new(100),
        },
        &[],
    )
    .unwrap();

    SetupTestResponse {
        app,
        flash_loan,
        receiver,
    }
}

#[test]
fn test_simple_loan() {
    let SetupTestResponse {
        mut app,
        flash_loan,
        receiver,
    } = setup_test(
        Uint128::new(100),
        Uint128::new(1),
        Uint128::new(101),
        Decimal::percent(1),
    );

    app.execute_contract(
        Addr::unchecked(CREATOR_ADDR),
        flash_loan.clone(),
        &ExecuteMsg::Loan {
            receiver: receiver.to_string(),
            amount: Uint128::new(100),
        },
        &[],
    )
    .unwrap();
}

#[test]
fn test_simple_loan_no_return() {
    let SetupTestResponse {
        mut app,
        flash_loan,
        receiver,
    } = setup_test(
        Uint128::new(100),
        Uint128::new(1),
        Uint128::new(100),
        Decimal::percent(1),
    );

    let err: ContractError = app
        .execute_contract(
            Addr::unchecked(CREATOR_ADDR),
            flash_loan.clone(),
            &ExecuteMsg::Loan {
                receiver: receiver.to_string(),
                amount: Uint128::new(100),
            },
            &[],
        )
        .unwrap_err()
        .downcast()
        .unwrap();

    assert!(matches!(err, ContractError::NotReturned {}))
}

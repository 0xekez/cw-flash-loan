use cosmwasm_std::{Addr, Coin, Decimal, Empty, Uint128};
use cw_multi_test::{App, BankSudo, Contract, ContractWrapper, Executor, SudoMsg};

use crate::msg::{ExecuteMsg, InstantiateMsg};

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

#[test]
fn test_simple_loan() {
    let mut app = App::default();

    let flash_code = app.store_code(flash_loan_contract());
    let receiver_code = app.store_code(simple_receiver_contract());

    let flash_loan = app
        .instantiate_contract(
            flash_code,
            Addr::unchecked(CREATOR_ADDR),
            &InstantiateMsg {
                admin: Some(CREATOR_ADDR.to_string()),
                fee: Decimal::percent(1),
                loan_denom: "ujuno".to_string(),
            },
            &[],
            "flash-loan",
            None,
        )
        .unwrap();

    let receiver = app
        .instantiate_contract(
            receiver_code,
            Addr::unchecked(CREATOR_ADDR),
            &cw_simple_loan_receiver::msg::InstantiateMsg {
                amount: Uint128::new(101),
                denom: "ujuno".to_string(),
            },
            &[],
            "receiver",
            None,
        )
        .unwrap();

    // Give the receiver enough funds to pay back a loan.
    app.sudo(SudoMsg::Bank(BankSudo::Mint {
        amount: vec![Coin {
            amount: Uint128::new(1),
            denom: "ujuno".to_string(),
        }],
        to_address: receiver.to_string(),
    }))
    .unwrap();

    // Give the lending contract enough funds to provide the loan.
    app.sudo(SudoMsg::Bank(BankSudo::Mint {
        amount: vec![Coin {
            amount: Uint128::new(100),
            denom: "ujuno".to_string(),
        }],
        to_address: flash_loan.to_string(),
    }))
    .unwrap();

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

    let final_balance = app.wrap().query_balance(flash_loan, "ujuno").unwrap();
    assert_eq!(final_balance.amount, Uint128::new(101));
}

#[test]
fn test_failed_loan() {
    let mut app = App::default();

    let flash_code = app.store_code(flash_loan_contract());
    let receiver_code = app.store_code(simple_receiver_contract());

    let flash_loan = app
        .instantiate_contract(
            flash_code,
            Addr::unchecked(CREATOR_ADDR),
            &InstantiateMsg {
                admin: Some(CREATOR_ADDR.to_string()),
                fee: Decimal::percent(1),
                loan_denom: "ujuno".to_string(),
            },
            &[],
            "flash-loan",
            None,
        )
        .unwrap();

    // The receiver will not be able to return the loan due to
    // insufficent balance.
    let receiver = app
        .instantiate_contract(
            receiver_code,
            Addr::unchecked(CREATOR_ADDR),
            &cw_simple_loan_receiver::msg::InstantiateMsg {
                amount: Uint128::new(10),
                denom: "ujuno".to_string(),
            },
            &[],
            "receiver",
            None,
        )
        .unwrap();

    // Give the lending contract enough funds to provide the loan.
    app.sudo(SudoMsg::Bank(BankSudo::Mint {
        amount: vec![Coin {
            amount: Uint128::new(100),
            denom: "ujuno".to_string(),
        }],
        to_address: flash_loan.to_string(),
    }))
    .unwrap();

    app.execute_contract(
        Addr::unchecked(CREATOR_ADDR),
        flash_loan.clone(),
        &ExecuteMsg::Loan {
            receiver: receiver.to_string(),
            amount: Uint128::new(100),
        },
        &[],
    )
    .unwrap_err();

    let receiver_balance = app.wrap().query_balance(receiver, "ujuno").unwrap();
    assert_eq!(receiver_balance.amount, Uint128::new(0));

    let final_balance = app.wrap().query_balance(flash_loan, "ujuno").unwrap();
    assert_eq!(final_balance.amount, Uint128::new(100));
}

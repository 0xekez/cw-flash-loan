use cosmwasm_std::{to_binary, Addr, Decimal, Empty, Uint128};
use cw20::Cw20Coin;
use cw_multi_test::{App, Contract, ContractWrapper, Executor};

use crate::{
    msg::{ExecuteMsg, InstantiateMsg, LoanDenom, QueryMsg},
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
    cw20: Addr,
}

fn setup_test(
    flash_balance: Uint128,
    receiver_balance: Uint128,
    receiver_return_amount: Uint128,
    fee: Decimal,
    mut initial_balances: Vec<Cw20Coin>,
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

    initial_balances.push(Cw20Coin {
        address: CREATOR_ADDR.to_string(),
        amount: flash_balance,
    });
    initial_balances.push(Cw20Coin {
        address: receiver.to_string(),
        amount: receiver_balance,
    });

    let cw20 = app
        .instantiate_contract(
            cw20_code,
            Addr::unchecked(CREATOR_ADDR),
            &cw20_base::msg::InstantiateMsg {
                name: "Floob Token".to_string(),
                symbol: "FLOOB".to_string(),
                decimals: 6,
                initial_balances,
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

    if !flash_balance.is_zero() {
        app.execute_contract(
            Addr::unchecked(CREATOR_ADDR),
            cw20.clone(),
            &cw20::Cw20ExecuteMsg::Transfer {
                recipient: flash_loan.to_string(),
                amount: flash_balance,
            },
            &[],
        )
        .unwrap();
    }

    SetupTestResponse {
        app,
        flash_loan,
        receiver,
        cw20,
    }
}

#[test]
fn test_simple_loan() {
    let SetupTestResponse {
        mut app,
        flash_loan,
        receiver,
        ..
    } = setup_test(
        Uint128::new(100),
        Uint128::new(1),
        Uint128::new(101),
        Decimal::percent(1),
        vec![],
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
        ..
    } = setup_test(
        Uint128::new(100),
        Uint128::new(1),
        Uint128::new(100),
        Decimal::percent(1),
        vec![],
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

#[test]
fn test_provider_rewards() {
    let SetupTestResponse {
        mut app,
        flash_loan,
        receiver,
        cw20,
    } = setup_test(
        Uint128::new(0),
        Uint128::new(100),
        Uint128::new(200),
        Decimal::percent(100),
        (0..10)
            .into_iter()
            .map(|i| Cw20Coin {
                address: format!("address_{}", i),
                amount: Uint128::new(10),
            })
            .collect(),
    );

    for i in 0..10 {
        let address = format!("address_{}", i);
        app.execute_contract(
            Addr::unchecked(&address),
            cw20.clone(),
            &cw20::Cw20ExecuteMsg::Send {
                contract: flash_loan.to_string(),
                amount: Uint128::new(10),
                msg: to_binary("").unwrap(),
            },
            &[],
        )
        .unwrap();

        let provided: Uint128 = app
            .wrap()
            .query_wasm_smart(flash_loan.clone(), &QueryMsg::Provided { address })
            .unwrap();
        assert_eq!(provided, Uint128::new(10));
    }

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

    // Withdraw. All the addresses should now have 20 floob.
    for i in 0..10 {
        let address = format!("address_{}", i);

        app.execute_contract(
            Addr::unchecked(&address),
            flash_loan.clone(),
            &ExecuteMsg::Withdraw {},
            &[],
        )
        .unwrap();

        let balance: cw20::BalanceResponse = app
            .wrap()
            .query_wasm_smart(cw20.clone(), &cw20::Cw20QueryMsg::Balance { address })
            .unwrap();
        assert_eq!(balance.balance, Uint128::new(20));
    }
}

#[test]
fn test_withdraw_no_provision() {
    let SetupTestResponse {
        mut app,
        flash_loan,
        ..
    } = setup_test(
        Uint128::new(100),
        Uint128::new(1),
        Uint128::new(100),
        Decimal::percent(1),
        vec![],
    );

    let err: ContractError = app
        .execute_contract(
            Addr::unchecked(CREATOR_ADDR),
            flash_loan,
            &ExecuteMsg::Withdraw {},
            &[],
        )
        .unwrap_err()
        .downcast()
        .unwrap();

    assert!(matches!(err, ContractError::NoProvisions {}))
}

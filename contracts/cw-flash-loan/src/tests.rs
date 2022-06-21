use cosmwasm_std::{Addr, Coin, Decimal, Empty, Uint128};
use cw_multi_test::{App, BankSudo, Contract, ContractWrapper, Executor, SudoMsg};

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
                loan_denom: LoanDenom::Native {
                    denom: "ujuno".to_string(),
                },
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
                loan_denom: LoanDenom::Native {
                    denom: "ujuno".to_string(),
                },
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

#[test]
fn test_provider_rewards() {
    let mut app = App::default();

    let flash_code = app.store_code(flash_loan_contract());
    let receiver_code = app.store_code(simple_receiver_contract());

    let flash_loan = app
        .instantiate_contract(
            flash_code,
            Addr::unchecked(CREATOR_ADDR),
            &InstantiateMsg {
                admin: Some(CREATOR_ADDR.to_string()),
                fee: Decimal::percent(100),
                loan_denom: LoanDenom::Native {
                    denom: "ujuno".to_string(),
                },
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
                amount: Uint128::new(200),
                denom: "ujuno".to_string(),
            },
            &[],
            "receiver",
            None,
        )
        .unwrap();

    // Give the receiving contract enough funds to repay the loan +
    // fee.
    app.sudo(SudoMsg::Bank(BankSudo::Mint {
        amount: vec![Coin {
            amount: Uint128::new(200),
            denom: "ujuno".to_string(),
        }],
        to_address: receiver.to_string(),
    }))
    .unwrap();

    // We will provide a total of 100 ujuno in liquidity split evenly
    // among ten addresses.
    for i in 0..10 {
        let address = format!("address_{}", i);

        app.sudo(SudoMsg::Bank(BankSudo::Mint {
            amount: vec![Coin {
                amount: Uint128::new(10),
                denom: "ujuno".to_string(),
            }],
            to_address: address.to_string(),
        }))
        .unwrap();

        app.execute_contract(
            Addr::unchecked(&address),
            flash_loan.clone(),
            &ExecuteMsg::Provide {},
            &[Coin {
                amount: Uint128::new(10),
                denom: "ujuno".to_string(),
            }],
        )
        .unwrap();
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

    // Withdraw. All the addresses should now have 20 ujuno.
    for i in 0..10 {
        let address = format!("address_{}", i);

        app.execute_contract(
            Addr::unchecked(&address),
            flash_loan.clone(),
            &ExecuteMsg::Withdraw {},
            &[],
        )
        .unwrap();

        let balance = app
            .wrap()
            .query_balance(address, "ujuno".to_string())
            .unwrap();
        assert_eq!(balance.amount, Uint128::new(20));
    }
}

#[test]
fn test_withdraw_no_provision() {
    let mut app = App::default();

    let flash_code = app.store_code(flash_loan_contract());

    let flash_loan = app
        .instantiate_contract(
            flash_code,
            Addr::unchecked(CREATOR_ADDR),
            &InstantiateMsg {
                admin: Some(CREATOR_ADDR.to_string()),
                fee: Decimal::percent(100),
                loan_denom: LoanDenom::Native {
                    denom: "ujuno".to_string(),
                },
            },
            &[],
            "flash-loan",
            None,
        )
        .unwrap();

    let first = "first".to_string();
    let second = "second".to_string();

    app.sudo(SudoMsg::Bank(BankSudo::Mint {
        amount: vec![Coin {
            amount: Uint128::new(100),
            denom: "ujuno".to_string(),
        }],
        to_address: first.clone(),
    }))
    .unwrap();

    app.execute_contract(
        Addr::unchecked(&first),
        flash_loan.clone(),
        &ExecuteMsg::Provide {},
        &[Coin {
            amount: Uint128::new(100),
            denom: "ujuno".to_string(),
        }],
    )
    .unwrap();

    let err: ContractError = app
        .execute_contract(
            Addr::unchecked(&second),
            flash_loan.clone(),
            &ExecuteMsg::Withdraw {},
            &[],
        )
        .unwrap_err()
        .downcast()
        .unwrap();
    assert!(matches!(err, ContractError::NoProvisions {}))
}

#[test]
fn test_withdraw_rounding() {
    let mut app = App::default();

    let flash_code = app.store_code(flash_loan_contract());

    let flash_loan = app
        .instantiate_contract(
            flash_code,
            Addr::unchecked(CREATOR_ADDR),
            &InstantiateMsg {
                admin: Some(CREATOR_ADDR.to_string()),
                fee: Decimal::percent(100),

                loan_denom: LoanDenom::Native {
                    denom: "ujuno".to_string(),
                },
            },
            &[],
            "flash-loan",
            None,
        )
        .unwrap();

    let first = "first".to_string();
    let second = "second".to_string();

    app.sudo(SudoMsg::Bank(BankSudo::Mint {
        amount: vec![Coin {
            amount: Uint128::new(100),
            denom: "ujuno".to_string(),
        }],
        to_address: first.clone(),
    }))
    .unwrap();

    app.execute_contract(
        Addr::unchecked(&first),
        flash_loan.clone(),
        &ExecuteMsg::Provide {},
        &[Coin {
            amount: Uint128::new(100),
            denom: "ujuno".to_string(),
        }],
    )
    .unwrap();

    app.sudo(SudoMsg::Bank(BankSudo::Mint {
        amount: vec![Coin {
            amount: Uint128::new(101),
            denom: "ujuno".to_string(),
        }],
        to_address: second.clone(),
    }))
    .unwrap();

    app.execute_contract(
        Addr::unchecked(&second),
        flash_loan.clone(),
        &ExecuteMsg::Provide {},
        &[Coin {
            amount: Uint128::new(101),
            denom: "ujuno".to_string(),
        }],
    )
    .unwrap();

    // Mint one token for the flash loan contract to make the math
    // screwy.
    app.sudo(SudoMsg::Bank(BankSudo::Mint {
        amount: vec![Coin {
            amount: Uint128::new(1),
            denom: "ujuno".to_string(),
        }],
        to_address: flash_loan.to_string(),
    }))
    .unwrap();

    // Should result in 101 being returned. If we rounded up would
    // result in 102 being returned.
    app.execute_contract(
        Addr::unchecked(&second),
        flash_loan.clone(),
        &ExecuteMsg::Withdraw {},
        &[],
    )
    .unwrap();
    let balance = app
        .wrap()
        .query_balance(second, "ujuno".to_string())
        .unwrap();
    assert_eq!(balance.amount, Uint128::new(101));

    // Should now also result in 101 being returned as this address
    // controlls 100% of current provisions.
    app.execute_contract(
        Addr::unchecked(&first),
        flash_loan.clone(),
        &ExecuteMsg::Withdraw {},
        &[],
    )
    .unwrap();
    let balance = app
        .wrap()
        .query_balance(first, "ujuno".to_string())
        .unwrap();
    assert_eq!(balance.amount, Uint128::new(101));
}

#[test]
fn test_adversarial_withdraw() {
    let mut app = App::default();

    let flash_code = app.store_code(flash_loan_contract());

    let flash_loan = app
        .instantiate_contract(
            flash_code,
            Addr::unchecked(CREATOR_ADDR),
            &InstantiateMsg {
                admin: Some(CREATOR_ADDR.to_string()),
                fee: Decimal::percent(100),
                loan_denom: LoanDenom::Native {
                    denom: "ujuno".to_string(),
                },
            },
            &[],
            "flash-loan",
            None,
        )
        .unwrap();

    let first = "first".to_string();
    let second = "second".to_string();

    app.sudo(SudoMsg::Bank(BankSudo::Mint {
        amount: vec![Coin {
            amount: Uint128::new(100),
            denom: "ujuno".to_string(),
        }],
        to_address: first.clone(),
    }))
    .unwrap();

    app.execute_contract(
        Addr::unchecked(&first),
        flash_loan.clone(),
        &ExecuteMsg::Provide {},
        &[Coin {
            amount: Uint128::new(100),
            denom: "ujuno".to_string(),
        }],
    )
    .unwrap();

    app.sudo(SudoMsg::Bank(BankSudo::Mint {
        amount: vec![Coin {
            amount: Uint128::new(101),
            denom: "ujuno".to_string(),
        }],
        to_address: second.clone(),
    }))
    .unwrap();

    app.execute_contract(
        Addr::unchecked(&second),
        flash_loan.clone(),
        &ExecuteMsg::Provide {},
        &[Coin {
            amount: Uint128::new(101),
            denom: "ujuno".to_string(),
        }],
    )
    .unwrap();

    // Mint two tokens for the flash loan contract to make the math
    // screwy.
    app.sudo(SudoMsg::Bank(BankSudo::Mint {
        amount: vec![Coin {
            amount: Uint128::new(2),
            denom: "ujuno".to_string(),
        }],
        to_address: flash_loan.to_string(),
    }))
    .unwrap();

    // Should result in 102 being returned.
    app.execute_contract(
        Addr::unchecked(&second),
        flash_loan.clone(),
        &ExecuteMsg::Withdraw {},
        &[],
    )
    .unwrap();
    let balance = app
        .wrap()
        .query_balance(second.clone(), "ujuno".to_string())
        .unwrap();
    assert_eq!(balance.amount, Uint128::new(102));

    // Now, re-provide those tokens to increase the share of the pool.
    app.execute_contract(
        Addr::unchecked(&second),
        flash_loan.clone(),
        &ExecuteMsg::Provide {},
        &[Coin {
            amount: Uint128::new(102),
            denom: "ujuno".to_string(),
        }],
    )
    .unwrap();

    // Re-providing should not increase entitled amount. In this case
    // we get rounded down.
    let entitled: Uint128 = app
        .wrap()
        .query_wasm_smart(flash_loan, &QueryMsg::Entitled { address: second })
        .unwrap();

    assert_eq!(entitled, Uint128::new(101))
}

#[test]
fn test_rewards_drain() {
    let mut app = App::default();

    let flash_code = app.store_code(flash_loan_contract());

    let flash_loan = app
        .instantiate_contract(
            flash_code,
            Addr::unchecked(CREATOR_ADDR),
            &InstantiateMsg {
                admin: Some(CREATOR_ADDR.to_string()),
                fee: Decimal::percent(100),
                loan_denom: LoanDenom::Native {
                    denom: "ujuno".to_string(),
                },
            },
            &[],
            "flash-loan",
            None,
        )
        .unwrap();

    let first = "first".to_string();
    let second = "second".to_string();

    app.sudo(SudoMsg::Bank(BankSudo::Mint {
        amount: vec![Coin {
            amount: Uint128::new(100),
            denom: "ujuno".to_string(),
        }],
        to_address: first.clone(),
    }))
    .unwrap();

    app.execute_contract(
        Addr::unchecked(&first),
        flash_loan.clone(),
        &ExecuteMsg::Provide {},
        &[Coin {
            amount: Uint128::new(100),
            denom: "ujuno".to_string(),
        }],
    )
    .unwrap();

    app.sudo(SudoMsg::Bank(BankSudo::Mint {
        amount: vec![Coin {
            amount: Uint128::new(100),
            denom: "ujuno".to_string(),
        }],
        to_address: second.clone(),
    }))
    .unwrap();

    app.execute_contract(
        Addr::unchecked(&second),
        flash_loan.clone(),
        &ExecuteMsg::Provide {},
        &[Coin {
            amount: Uint128::new(100),
            denom: "ujuno".to_string(),
        }],
    )
    .unwrap();

    // Tons of flash loans happen and now the contract has a lot of
    // money.
    app.sudo(SudoMsg::Bank(BankSudo::Mint {
        amount: vec![Coin {
            amount: Uint128::new(200),
            denom: "ujuno".to_string(),
        }],
        to_address: flash_loan.to_string(),
    }))
    .unwrap();

    // Address withdraws and collects rewards.
    app.execute_contract(
        Addr::unchecked(&first),
        flash_loan.clone(),
        &ExecuteMsg::Withdraw {},
        &[],
    )
    .unwrap();
    let balance = app
        .wrap()
        .query_balance(first.clone(), "ujuno".to_string())
        .unwrap();
    assert_eq!(balance.amount, Uint128::new(200));

    // Now, address provides again with rewards + initial amount.
    app.execute_contract(
        Addr::unchecked(&first),
        flash_loan.clone(),
        &ExecuteMsg::Provide {},
        &[Coin {
            amount: Uint128::new(200),
            denom: "ujuno".to_string(),
        }],
    )
    .unwrap();

    // Address withdraws. They should not receive additional tokens.
    app.execute_contract(
        Addr::unchecked(&first),
        flash_loan.clone(),
        &ExecuteMsg::Withdraw {},
        &[],
    )
    .unwrap();
    let balance = app
        .wrap()
        .query_balance(first.clone(), "ujuno".to_string())
        .unwrap();
    assert_eq!(balance.amount, Uint128::new(200));
}

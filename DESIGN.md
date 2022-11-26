

# Design Proposal

Found [vulnerability](https://twitter.com/0xekez/status/1596180033185280000?s=20&t=FNdqjzdAeicj4BN-Vzq9wg) that would drain funds. 

Re-design proposal: add a new state variable `UNPAID_FLASHLOAN` that increments on `Loan` message
and decrements on `RepayLoan` message. Modify interface for execute messages to fix vulnerability
along with some design improvements:

```rust
#[cw_serde]
pub enum ExecuteMsg {
    UpdateConfig { admin: Option<String>, fee: Decimal },
    Loan { receiver: String, amount: Uint128 },
    // Fix to vulnerability
	RepayLoan { amount: Uint128 }, 
    AssertBalance { amount: Uint128 },
    // Single provision method for both denominations
    Provide { amount: Uint128 },
    // Partial withdrawal 
    Withdraw { amount: Uint128 },
}
```

Summary of changes for vulnerability:
- Add `RepayLoan` message

Summary of changes for cleaner design:
- Remove CW-20 `Receive` message in favor of a single Provide and allowance-based CW-20 transfer
- Add `amount` attribute to `Provide` (for CW-20 transfer) and `Withdraw` message (for partial withdrawals)

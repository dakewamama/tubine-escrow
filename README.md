# Turbin3 Builders Q4 2026: Week 2

Vault and Escrow in Anchor, with Rust integration tests using LiteSVM.

This repository is based on the Turbin3 escrow starter by [ShrinathNR](https://github.com/ShrinathNR/tubine-escrow). The Week 2 implementation adds the missing escrow `take` and `update` handlers, explicit validation, a standalone SOL vault, and tests.

## Programs

| Program | Instructions | Purpose |
| --- | --- | --- |
| `week2_vault` | `initialize`, `deposit`, `withdraw`, `close` | Per-wallet SOL vault with a state PDA and System Program-owned vault PDA. |
| `escrowq32026` | `make`, `take`, `refund`, `update` | Atomic exchange of token A for token B, with maker-directed updates and an expiry for `take`. |

## Build and tests

The starter uses Anchor 1.1.2, Rust 1.89.0 and LiteSVM 0.10.0. Install matching toolchains before building. Program tests load the SBF artifacts from `target/deploy/`.

```bash
anchor build
cargo test --workspace -- --nocapture
```

Use a development wallet and local test mints. **Do not fund these programs with real-value assets.** This is educational code and has not been audited.

The passing-test screenshot required by Classroom must be captured from an actual successful run; a test command in a README is not proof that the tests passed.

## SOL vault

The vault is defined by two PDAs derived from the owner public key:

- `state = PDA(["state", owner])`: stores owner and PDA bumps; created by `initialize`.
- `vault = PDA(["vault", owner])`: an empty-data account owned by the System Program; holds native SOL.

`initialize` funds the vault at the zero-data rent-exempt minimum. `deposit` transfers SOL from the signing owner. `withdraw` requires the same owner and preserves the vault reserve. `close` transfers all remaining SOL to the owner, then closes the state account and returns its rent.

Vault invariants: zero-value transfers are rejected; a different wallet cannot supply the owner's PDA; an ordinary withdraw cannot reduce the vault below its reserve; `close` is available only to the owner.

## Token escrow

A maker locks `deposit` units of mint A and offers them for `receive` units of mint B. The escrow identity is derived from the maker public key and a maker-chosen `u64` seed.

- `make(seed, deposit, receive, expiration)`: requires two different mints, positive amounts and a future expiry; stores terms and moves mint A into the program-controlled vault.
- `take()`: callable by a taker strictly before expiry. In one atomic transaction, sends the requested mint B to the maker and the vault's mint A to the taker. Closes the vault and escrow afterward. Missing recipient token accounts can be initialized by the taker.
- `refund()`: maker-only cancellation, available while the escrow exists. Returns mint A to the maker and closes the accounts. This is a deliberate cancellation rule, not an automatic timeout refund.
- `update(receive, expiration)`: maker-only change to the amount of mint B requested and future expiry. The deposited asset, locked amount, maker and mint identities do not change.

The Clock sysvar gates `make`, `take` and `update`. A taker who submits after expiry fails. An escrow that expires remains recoverable by its maker via `refund`; there is no background keeper. In all flows, SPL Token enforces mint, token-account and signer constraints. `take` is atomic: if any transfer fails, neither leg is committed.

## Test plan

The LiteSVM tests exercise the four Vault instructions and all four Escrow instructions. The key checks are:

| Group | Assertions |
| --- | --- |
| Vault setup | state ownership and PDA derivation; duplicate initialization rejected |
| Vault deposit / withdraw | precise lamport deltas; zero and over-withdraws rejected |
| Vault close | state closed; SOL returned |
| Escrow make | immutable maker/mint fields; vault amount; invalid amounts; duplicate seed rejected |
| Escrow take | both token transfers and account closure; repeated take rejected |
| Escrow refund | maker token balance restored; repeated refund rejected |
| Escrow update | price/deadline changes only for maker; invalid update rejected |

Further adversarial coverage should include a tampered mint or destination account, expired take using a warped Clock sysvar, and a second-wallet attempt to withdraw someone else's vault. Run and record these before declaring the code production-ready.

## Project scope and credits

This is an individual Week 2 exercise. It is not StateRail, and it does not implement a production-ready escrow service or the advanced in-kind vault extension. Starter source and program structure are credited to [ShrinathNR/tubine-escrow](https://github.com/ShrinathNR/tubine-escrow); completed handler and test changes should be visible in the fork's commit history.

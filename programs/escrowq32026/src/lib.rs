pub mod constants;
pub mod error;
pub mod instructions;
pub mod state;

use anchor_lang::prelude::*;
pub use constants::*;
pub use instructions::*;
pub use state::*;

declare_id!("5Y6HMSgNYbkcBiQCukYvTK56aQarSpq1Nk9aiSsjws2o");

#[program]
pub mod escrowq32026 {
    use super::*;

    #[instruction(discriminator = 0)]
    pub fn make(
        ctx: Context<Make>,
        seed: u64,
        deposit: u64,
        receive: u64,
        expiration: i64,
    ) -> Result<()> {
        require!(deposit > 0 && receive > 0, EscrowError::InvalidAmount);
        require!(ctx.accounts.mint_a.key() != ctx.accounts.mint_b.key(), EscrowError::SameMint);
        require!(expiration > Clock::get()?.unix_timestamp, EscrowError::InvalidExpiration);
        ctx.accounts.init_escrow(seed, receive, expiration, &ctx.bumps)?;
        ctx.accounts.deposit(deposit)
    }

    #[instruction(discriminator = 1)]
    pub fn take(ctx: Context<Take>) -> Result<()> {
        require!(Clock::get()?.unix_timestamp < ctx.accounts.escrow.expiration, EscrowError::Expired);
        ctx.accounts.execute_swap()
    }

    #[instruction(discriminator = 2)]
    pub fn refund(ctx: Context<Refund>) -> Result<()> {
        ctx.accounts.refund_and_close()
    }

    #[instruction(discriminator = 3)]
    pub fn update(ctx: Context<Update>, receive: u64, expiration: i64) -> Result<()> {
        require!(receive > 0, EscrowError::InvalidAmount);
        require!(expiration > Clock::get()?.unix_timestamp, EscrowError::InvalidExpiration);
        ctx.accounts.escrow.receive = receive;
        ctx.accounts.escrow.expiration = expiration;
        Ok(())
    }
}

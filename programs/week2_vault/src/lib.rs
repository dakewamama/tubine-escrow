use anchor_lang::prelude::*;
use anchor_lang::system_program::{transfer, Transfer};

declare_id!("7A4QgWQJrrpuPnSMavTVsrjpLiVq451vucm2CeLyoGrH");

const STATE_SEED: &[u8] = b"state";
const VAULT_SEED: &[u8] = b"vault";

#[program]
pub mod week2_vault {
    use super::*;

    pub fn initialize(ctx: Context<Initialize>) -> Result<()> {
        let user = ctx.accounts.user.key();
        let state = &mut ctx.accounts.state;
        state.owner = user;
        state.state_bump = ctx.bumps.state;
        state.vault_bump = ctx.bumps.vault;

        // A funded system account, not an arbitrary account owned by this program.
        let reserve = Rent::get()?.minimum_balance(0);
        transfer(
            CpiContext::new(
                ctx.accounts.system_program.to_account_info(),
                Transfer {
                    from: ctx.accounts.user.to_account_info(),
                    to: ctx.accounts.vault.to_account_info(),
                },
            ),
            reserve,
        )?;
        Ok(())
    }

    pub fn deposit(ctx: Context<Deposit>, amount: u64) -> Result<()> {
        require!(amount > 0, VaultError::ZeroAmount);
        transfer(
            CpiContext::new(
                ctx.accounts.system_program.to_account_info(),
                Transfer {
                    from: ctx.accounts.user.to_account_info(),
                    to: ctx.accounts.vault.to_account_info(),
                },
            ),
            amount,
        )
    }

    pub fn withdraw(ctx: Context<Withdraw>, amount: u64) -> Result<()> {
        require!(amount > 0, VaultError::ZeroAmount);
        let reserve = Rent::get()?.minimum_balance(0);
        let balance = ctx.accounts.vault.lamports();
        require!(
            balance.checked_sub(amount).ok_or(VaultError::InsufficientFunds)? >= reserve,
            VaultError::InsufficientFunds
        );

        let signer_seeds: &[&[&[u8]]] = &[&[
            VAULT_SEED,
            ctx.accounts.user.key.as_ref(),
            &[ctx.accounts.state.vault_bump],
        ]];
        transfer(
            CpiContext::new_with_signer(
                ctx.accounts.system_program.to_account_info(),
                Transfer {
                    from: ctx.accounts.vault.to_account_info(),
                    to: ctx.accounts.user.to_account_info(),
                },
                signer_seeds,
            ),
            amount,
        )
    }

    pub fn close(ctx: Context<Close>) -> Result<()> {
        let balance = ctx.accounts.vault.lamports();
        let signer_seeds: &[&[&[u8]]] = &[&[
            VAULT_SEED,
            ctx.accounts.user.key.as_ref(),
            &[ctx.accounts.state.vault_bump],
        ]];
        // The last transfer reclaims the reserve; Anchor also returns state rent.
        transfer(
            CpiContext::new_with_signer(
                ctx.accounts.system_program.to_account_info(),
                Transfer {
                    from: ctx.accounts.vault.to_account_info(),
                    to: ctx.accounts.user.to_account_info(),
                },
                signer_seeds,
            ),
            balance,
        )
    }
}

#[account]
#[derive(InitSpace)]
pub struct VaultState {
    pub owner: Pubkey,
    pub state_bump: u8,
    pub vault_bump: u8,
}

#[derive(Accounts)]
pub struct Initialize<'info> {
    #[account(mut)]
    pub user: Signer<'info>,
    #[account(
        init,
        payer = user,
        seeds = [STATE_SEED, user.key().as_ref()],
        bump,
        space = 8 + VaultState::INIT_SPACE
    )]
    pub state: Account<'info, VaultState>,
    /// CHECK: This is a deterministic System Program account funded by initialize.
    #[account(mut, seeds = [VAULT_SEED, user.key().as_ref()], bump)]
    pub vault: UncheckedAccount<'info>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct Deposit<'info> {
    #[account(mut)]
    pub user: Signer<'info>,
    #[account(
        seeds = [STATE_SEED, user.key().as_ref()],
        bump = state.state_bump,
        constraint = state.owner == user.key() @ VaultError::InvalidOwner
    )]
    pub state: Account<'info, VaultState>,
    /// CHECK: Address and System ownership are both constrained.
    #[account(mut, seeds = [VAULT_SEED, user.key().as_ref()], bump = state.vault_bump, owner = System::id())]
    pub vault: UncheckedAccount<'info>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct Withdraw<'info> {
    #[account(mut)]
    pub user: Signer<'info>,
    #[account(
        seeds = [STATE_SEED, user.key().as_ref()],
        bump = state.state_bump,
        constraint = state.owner == user.key() @ VaultError::InvalidOwner
    )]
    pub state: Account<'info, VaultState>,
    /// CHECK: Address and System ownership are both constrained.
    #[account(mut, seeds = [VAULT_SEED, user.key().as_ref()], bump = state.vault_bump, owner = System::id())]
    pub vault: UncheckedAccount<'info>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct Close<'info> {
    #[account(mut)]
    pub user: Signer<'info>,
    #[account(
        mut,
        close = user,
        seeds = [STATE_SEED, user.key().as_ref()],
        bump = state.state_bump,
        constraint = state.owner == user.key() @ VaultError::InvalidOwner
    )]
    pub state: Account<'info, VaultState>,
    /// CHECK: Address and System ownership are both constrained.
    #[account(mut, seeds = [VAULT_SEED, user.key().as_ref()], bump = state.vault_bump, owner = System::id())]
    pub vault: UncheckedAccount<'info>,
    pub system_program: Program<'info, System>,
}

#[error_code]
pub enum VaultError {
    #[msg("Amount must be positive")]
    ZeroAmount,
    #[msg("Insufficient balance after preserving rent")]
    InsufficientFunds,
    #[msg("Only the vault owner can change the vault")]
    InvalidOwner,
}

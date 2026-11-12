use anchor_lang::prelude::*;

#[error_code]
pub enum EscrowError {
    #[msg("Deposit and receive amounts must be positive")]
    InvalidAmount,
    #[msg("Swap mints must be different")]
    SameMint,
    #[msg("Expiration must be in the future")]
    InvalidExpiration,
    #[msg("Escrow has expired")]
    Expired,
}

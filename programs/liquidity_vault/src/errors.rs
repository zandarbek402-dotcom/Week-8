use anchor_lang::prelude::*;

#[error_code]
pub enum VaultError {
    #[msg("Amount must be greater than zero")]
    ZeroAmount,
    #[msg("Vault is paused")]
    VaultPaused,
    #[msg("Deposit would exceed the vault deposit cap")]
    DepositCapExceeded,
    #[msg("Fee is too high")]
    FeeTooHigh,
    #[msg("Not enough shares")]
    InsufficientShares,
    #[msg("Calculated shares amount is zero")]
    ZeroShares,
    #[msg("Math overflow")]
    MathOverflow,
    #[msg("Only the vault admin can do this")]
    Unauthorized,
}

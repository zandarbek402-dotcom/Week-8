use anchor_lang::prelude::*;
use anchor_spl::{
    associated_token::AssociatedToken,
    token::{self, Mint, MintTo, Token, TokenAccount, Transfer},
};

use crate::{
    constants::*,
    errors::VaultError,
    math::shares_for_deposit,
    state::{UserPosition, VaultConfig},
};

#[derive(Accounts)]
pub struct Deposit<'info> {
    #[account(mut)]
    pub owner: Signer<'info>,

    #[account(
        mut,
        seeds = [CONFIG_SEED, config.underlying_mint.as_ref()],
        bump = config.bump,
        has_one = share_mint,
        has_one = vault_token_account,
        has_one = underlying_mint,
    )]
    pub config: Account<'info, VaultConfig>,

    pub underlying_mint: Account<'info, Mint>,

    #[account(mut)]
    pub share_mint: Account<'info, Mint>,

    #[account(mut)]
    pub vault_token_account: Account<'info, TokenAccount>,

    #[account(
        mut,
        seeds = [USER_SEED, config.key().as_ref(), owner.key().as_ref()],
        bump = user_position.bump,
        has_one = owner,
    )]
    pub user_position: Account<'info, UserPosition>,

    /// User's underlying tokens (source)
    #[account(
        mut,
        token::mint = underlying_mint,
        token::authority = owner,
    )]
    pub user_token_account: Account<'info, TokenAccount>,

    /// User's share tokens (destination), created if missing
    #[account(
        init_if_needed,
        payer = owner,
        associated_token::mint = share_mint,
        associated_token::authority = owner,
    )]
    pub user_share_account: Account<'info, TokenAccount>,

    pub token_program: Program<'info, Token>,
    pub associated_token_program: Program<'info, AssociatedToken>,
    pub system_program: Program<'info, System>,
}

pub fn handler(ctx: Context<Deposit>, amount: u64) -> Result<()> {
    require!(amount > 0, VaultError::ZeroAmount);
    let config = &ctx.accounts.config;
    require!(!config.paused, VaultError::VaultPaused);

    let total_assets = ctx.accounts.vault_token_account.amount;
    if config.deposit_cap > 0 {
        let after = total_assets
            .checked_add(amount)
            .ok_or(VaultError::MathOverflow)?;
        require!(after <= config.deposit_cap, VaultError::DepositCapExceeded);
    }

    let shares = shares_for_deposit(amount, ctx.accounts.share_mint.supply, total_assets)?;
    require!(shares > 0, VaultError::ZeroShares);

    // 1. user -> vault
    token::transfer(
        CpiContext::new(
            ctx.accounts.token_program.to_account_info(),
            Transfer {
                from: ctx.accounts.user_token_account.to_account_info(),
                to: ctx.accounts.vault_token_account.to_account_info(),
                authority: ctx.accounts.owner.to_account_info(),
            },
        ),
        amount,
    )?;

    // 2. mint shares to user, signed by config PDA
    let mint_key = config.underlying_mint;
    let bump = config.bump;
    let signer_seeds: &[&[&[u8]]] = &[&[CONFIG_SEED, mint_key.as_ref(), &[bump]]];
    token::mint_to(
        CpiContext::new_with_signer(
            ctx.accounts.token_program.to_account_info(),
            MintTo {
                mint: ctx.accounts.share_mint.to_account_info(),
                to: ctx.accounts.user_share_account.to_account_info(),
                authority: ctx.accounts.config.to_account_info(),
            },
            signer_seeds,
        ),
        shares,
    )?;

    // TODO (week 9+): CPI into the liquidity pool (add_liquidity) with vault funds

    // 3. accounting
    let config = &mut ctx.accounts.config;
    config.total_deposited = config
        .total_deposited
        .checked_add(amount)
        .ok_or(VaultError::MathOverflow)?;

    let user = &mut ctx.accounts.user_position;
    user.shares = user.shares.checked_add(shares).ok_or(VaultError::MathOverflow)?;
    user.deposited = user.deposited.checked_add(amount).ok_or(VaultError::MathOverflow)?;
    user.last_action_ts = Clock::get()?.unix_timestamp;

    msg!("Deposited {} tokens, minted {} shares", amount, shares);
    Ok(())
}

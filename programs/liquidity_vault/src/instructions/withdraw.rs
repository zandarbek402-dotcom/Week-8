use anchor_lang::prelude::*;
use anchor_spl::token::{self, Burn, Mint, Token, TokenAccount, Transfer};

use crate::{
    constants::*,
    errors::VaultError,
    math::assets_for_shares,
    state::{UserPosition, VaultConfig},
};

#[derive(Accounts)]
pub struct Withdraw<'info> {
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

    #[account(
        mut,
        token::mint = underlying_mint,
        token::authority = owner,
    )]
    pub user_token_account: Account<'info, TokenAccount>,

    #[account(
        mut,
        token::mint = share_mint,
        token::authority = owner,
    )]
    pub user_share_account: Account<'info, TokenAccount>,

    pub token_program: Program<'info, Token>,
}

pub fn handler(ctx: Context<Withdraw>, shares: u64) -> Result<()> {
    require!(shares > 0, VaultError::ZeroAmount);
    require!(!ctx.accounts.config.paused, VaultError::VaultPaused);
    require!(
        ctx.accounts.user_position.shares >= shares
            && ctx.accounts.user_share_account.amount >= shares,
        VaultError::InsufficientShares
    );

    let gross = assets_for_shares(
        shares,
        ctx.accounts.share_mint.supply,
        ctx.accounts.vault_token_account.amount,
    )?;
    // fee stays in the vault -> benefits the remaining share holders
    let fee = (gross as u128 * ctx.accounts.config.fee_bps as u128 / MAX_BPS as u128) as u64;
    let amount_out = gross.checked_sub(fee).ok_or(VaultError::MathOverflow)?;
    require!(amount_out > 0, VaultError::ZeroAmount);

    // TODO (week 9+): remove liquidity from the pool if vault balance is not enough

    // 1. burn user's shares
    token::burn(
        CpiContext::new(
            ctx.accounts.token_program.to_account_info(),
            Burn {
                mint: ctx.accounts.share_mint.to_account_info(),
                from: ctx.accounts.user_share_account.to_account_info(),
                authority: ctx.accounts.owner.to_account_info(),
            },
        ),
        shares,
    )?;

    // 2. vault -> user, signed by config PDA
    let mint_key = ctx.accounts.config.underlying_mint;
    let bump = ctx.accounts.config.bump;
    let signer_seeds: &[&[&[u8]]] = &[&[CONFIG_SEED, mint_key.as_ref(), &[bump]]];
    token::transfer(
        CpiContext::new_with_signer(
            ctx.accounts.token_program.to_account_info(),
            Transfer {
                from: ctx.accounts.vault_token_account.to_account_info(),
                to: ctx.accounts.user_token_account.to_account_info(),
                authority: ctx.accounts.config.to_account_info(),
            },
            signer_seeds,
        ),
        amount_out,
    )?;

    // 3. accounting
    let config = &mut ctx.accounts.config;
    config.total_deposited = config.total_deposited.saturating_sub(amount_out);

    let user = &mut ctx.accounts.user_position;
    user.shares -= shares;
    user.deposited = user.deposited.saturating_sub(amount_out);
    user.last_action_ts = Clock::get()?.unix_timestamp;

    msg!("Burned {} shares, withdrew {} (fee {})", shares, amount_out, fee);
    Ok(())
}

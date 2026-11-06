use anchor_lang::prelude::*;

pub mod constants;
pub mod errors;
pub mod instructions;
pub mod math;
pub mod state;

use instructions::*;

declare_id!("264Fck2jjphRCD3vZ5uKA9zzKnzhjXsqGjpNsY2YopeZ");

/// Week 8 — BUILD SPRINT: Liquidity Vault skeleton.
/// Users deposit tokens -> receive share tokens.
/// Next weeks: vault funds are placed into a liquidity pool.
#[program]
pub mod liquidity_vault {
    use super::*;

    /// Admin creates the vault: config PDA + share mint PDA + vault token account PDA
    pub fn initialize_vault(
        ctx: Context<InitializeVault>,
        fee_bps: u16,
        deposit_cap: u64,
    ) -> Result<()> {
        instructions::initialize_vault::handler(ctx, fee_bps, deposit_cap)
    }

    /// User creates their position PDA
    pub fn initialize_user(ctx: Context<InitializeUser>) -> Result<()> {
        instructions::initialize_user::handler(ctx)
    }

    /// Deposit underlying tokens, receive shares
    pub fn deposit(ctx: Context<Deposit>, amount: u64) -> Result<()> {
        instructions::deposit::handler(ctx, amount)
    }

    /// Burn shares, receive underlying tokens back
    pub fn withdraw(ctx: Context<Withdraw>, shares: u64) -> Result<()> {
        instructions::withdraw::handler(ctx, shares)
    }

    /// Admin: change fee / cap / pause
    pub fn update_config(
        ctx: Context<UpdateConfig>,
        fee_bps: Option<u16>,
        deposit_cap: Option<u64>,
        paused: Option<bool>,
    ) -> Result<()> {
        instructions::update_config::handler(ctx, fee_bps, deposit_cap, paused)
    }
}

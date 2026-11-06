use crate::errors::VaultError;
use anchor_lang::prelude::*;

/// shares = amount * total_shares / total_assets  (first deposit is 1:1)
pub fn shares_for_deposit(amount: u64, total_shares: u64, total_assets: u64) -> Result<u64> {
    if total_shares == 0 || total_assets == 0 {
        return Ok(amount);
    }
    let v = (amount as u128)
        .checked_mul(total_shares as u128)
        .ok_or(VaultError::MathOverflow)?
        / total_assets as u128;
    u64::try_from(v).map_err(|_| error!(VaultError::MathOverflow))
}

/// assets = shares * total_assets / total_shares
pub fn assets_for_shares(shares: u64, total_shares: u64, total_assets: u64) -> Result<u64> {
    require!(total_shares > 0, VaultError::InsufficientShares);
    let v = (shares as u128)
        .checked_mul(total_assets as u128)
        .ok_or(VaultError::MathOverflow)?
        / total_shares as u128;
    u64::try_from(v).map_err(|_| error!(VaultError::MathOverflow))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn first_deposit_is_one_to_one() {
        assert_eq!(shares_for_deposit(1_000, 0, 0).unwrap(), 1_000);
    }

    #[test]
    fn proportional_shares() {
        // vault grew 50% -> new depositor gets fewer shares per token
        assert_eq!(shares_for_deposit(150, 100, 150).unwrap(), 100);
        assert_eq!(assets_for_shares(100, 200, 300).unwrap(), 150);
    }
}

use anchor_lang::prelude::*;

declare_id!("2oxkz3u24B8YKFnfm1VvE1ydWfmiAyqQryT41eyk1G2B");

mod constants;
mod contexts;
mod errors;
mod helpers;
mod state;

use contexts::*;
pub use state::*;
pub use constants::*;


#[program]
pub mod anchor_amm {
    use super::*;

    pub fn initialize(
        ctx: Context<Initialize>,
        seed: u64,
        fee: u16,
        authority: Option<Pubkey>
    ) -> Result<()> {
        ctx.accounts.initialize(seed, fee, authority, &ctx.bumps)
    }

    pub fn deposit(
        ctx: Context<Deposit>,
        amount: u64,
        x_max: u64,
        y_max: u64,
        expiration: i64,
    ) -> Result<()> {
        ctx.accounts.deposit(amount, x_max, y_max, expiration)
    }

    pub fn lock(ctx: Context<Update>) -> Result<()> {
        ctx.accounts.lock()
    }

    pub fn unlock(ctx: Context<Update>) -> Result<()> {
        ctx.accounts.unlock()
    }

    pub fn swap(
        ctx: Context<Swap>,
        amount_in: u64,
        min_amount_out: u64,
        is_x_to_y: bool,
        expiration: i64
    ) -> Result<()> {
        ctx.accounts.swap(amount_in, min_amount_out, is_x_to_y, expiration)
    }

    pub fn withdraw(
        ctx: Context<Withdraw>,
        amount: u64,
        x_min: u64,
        y_min: u64,
        expiration: i64,
    ) -> Result<()> {
        ctx.accounts.withdraw(amount, x_min, y_min, expiration)
    }

}


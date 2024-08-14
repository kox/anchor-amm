use anchor_lang::prelude::*;

declare_id!("2oxkz3u24B8YKFnfm1VvE1ydWfmiAyqQryT41eyk1G2B");

mod constants;
mod contexts;
mod errors;
mod helpers;
mod state;

use contexts::*;
pub use errors::*;
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
/* 
    pub fn deposit(ctx: Context<Initialize>) -> Result<()> {
        msg!("Greetings from: {:?}", ctx.program_id);
        Ok(())
    }

    pub fn withdraw(ctx: Context<Initialize>) -> Result<()> {
        msg!("Greetings from: {:?}", ctx.program_id);
        Ok(())
    }

    pub fn swap(ctx: Context<Initialize>) -> Result<()> {
        msg!("Greetings from: {:?}", ctx.program_id);
        Ok(())
    }

    pub fn unlock(ctx: Context<Initialize>) -> Result<()> {
        msg!("Greetings from: {:?}", ctx.program_id);
        Ok(())
    } */
}


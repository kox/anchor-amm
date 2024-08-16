use anchor_lang::prelude::*;

use crate::{
    has_update_authority,  Config, 
    errors::AmmError
};

#[derive(Accounts)]
pub struct Update<'info> {
    #[account(mut)]
    pub payer: Signer<'info>,

    #[account(
        mut,
        seeds = [
            b"config", 
            config.seed.to_le_bytes().as_ref()
        ],
        bump = config.config_bump,
    )]
    pub config: Account<'info, Config>,

    system_account: Program<'info, System>,
}

impl<'info> Update<'info> {
    pub fn lock(&mut self) -> Result<()> {
        has_update_authority!(self);

        self.config.locked = true;

        Ok(())
    }

    pub fn unlock(&mut self) -> Result<()> {
        has_update_authority!(self);

        self.config.locked = false;

        Ok(())
    }
}

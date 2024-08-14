use anchor_lang::prelude::*;
use anchor_spl::{
    associated_token::AssociatedToken, 
    token::Token, 
    token_interface::{ Mint, TokenAccount }
};

use crate::{AmmError, Config};

/// Initialize Context
/// 
/// It will require to expose the seed to create the pools
#[derive(Accounts)]
#[instruction(seed: u64)]
pub struct Initialize<'info> {
    // As always we will need a person who will pay for creating the LP
    #[account(mut)]
    pub payer: Signer<'info>,

    // An AMM allows to exchange 2 different SPL tokens, therefore we will need to define both mint accounts
    pub x_mint: Box<InterfaceAccount<'info, Mint>>,
    pub y_mint: Box<InterfaceAccount<'info, Mint>>,

    // We will need ATAs to store X and Y tokens
    #[account(
        init,
        payer = payer,
        associated_token::mint = x_mint,
        associated_token::authority = auth,
    )]
    pub x_vault: Box<InterfaceAccount<'info, TokenAccount>>,
    
    #[account(
        init,
        payer = payer,
        associated_token::mint = y_mint,
        associated_token::authority = auth,
    )]
    pub y_vault: Box<InterfaceAccount<'info, TokenAccount>>,

    /// CHECK: This account is only used to sign. it doesn't contain SOL
    #[account(seeds = [b"auth"], bump)]
    pub auth: UncheckedAccount<'info>,

    // We will need an extra PDA to store some configuration
    #[account(
        init,
        payer = payer,
        seeds = [b"config", seed.to_le_bytes().as_ref()],
        bump,
        space = Config::INIT_SPACE
    )]
    pub config: Account<'info, Config>,

    // Last we will include the root programs to create accounts, tokens and ATAs
    pub token_program: Program<'info, Token>,
    pub associated_token_program: Program<'info, AssociatedToken>,
    pub system_program: Program<'info, System>,
}

impl<'info> Initialize<'info> {
    pub fn initialize(
        &mut self,
        seed: u64,
        fee: u16,
        authority: Option<Pubkey>,
        bumps: &InitializeBumps,
    ) -> Result<()> {
        // Fee can't be higher than 100%. We will  pass it without decimas 0-10000
        require!(fee <= 10000, AmmError::InvalidFee);

        self.config.init(
            seed,
            authority,
            self.x_mint.key(),
            self.y_mint.key(),
            fee,
            bumps.auth,
            bumps.config,
        );

        Ok(())
    }
}
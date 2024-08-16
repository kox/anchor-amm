use anchor_lang::prelude::*;
use anchor_spl::{
    associated_token::AssociatedToken,
    token_interface::{ Mint, TokenAccount },
    token::{ transfer, burn, Token, Transfer, Burn },
};

use crate::{
    assert_not_locked, assert_not_expired, assert_non_zero, Config,
    helpers::{ ConstantProduct },
    errors::AmmError,
};

#[derive(Accounts)]
pub struct Withdraw<'info> {
    // As usualy the person who pays/withdraw
    #[account(mut)]
    pub payer: Signer<'info>,

    // We keep specifying the both mint pubkeys for X, Y & LP
    pub x_mint: Box<InterfaceAccount<'info, Mint>>,
    pub y_mint: Box<InterfaceAccount<'info, Mint>>,
    #[account(
        mut,
        seeds = [b"lp", config.key().as_ref()],
        bump = config.lp_bump
    )]
    pub lp_mint: Box<InterfaceAccount<'info, Mint>>,
    
    // We include the vault accounts X, Y
    #[account(
        mut,
        associated_token::mint = x_mint,
        associated_token::authority = auth,
    )]
    pub x_vault: Box<InterfaceAccount<'info, TokenAccount>>,
    #[account(
        mut,
        associated_token::mint = y_mint,
        associated_token::authority = auth,
    )]
    pub y_vault: Box<InterfaceAccount<'info, TokenAccount>>,
    #[account(
        init_if_needed,
        payer = payer,
        associated_token::mint = x_mint,
        associated_token::authority = payer,
    )]
    pub x_user_ata: Box<InterfaceAccount<'info, TokenAccount>>,
    #[account(
        init_if_needed,
        payer = payer,
        associated_token::mint = y_mint,
        associated_token::authority = payer,
    )]
    pub y_user_ata: Box<InterfaceAccount<'info, TokenAccount>>,
    #[account(
        mut,
        associated_token::mint = lp_mint,
        associated_token::authority = payer,
    )]
    pub lp_user_ata: Box<InterfaceAccount<'info, TokenAccount>>,
    
    /// CHECK: just a pda for signing. no contains SOL
    #[account(seeds = [b"auth"], bump = config.auth_bump)]
    pub auth: UncheckedAccount<'info>,

    #[account(
        has_one = x_mint,
        has_one = y_mint,
        seeds = [
            b"config",
            config.seed.to_le_bytes().as_ref()
        ],
        bump = config.config_bump,
    )]
    pub config: Account<'info, Config>,
    
    pub token_program: Program<'info, Token>,
    pub associated_token_program: Program<'info, AssociatedToken>,
    pub system_program: Program<'info, System>,
}

impl<'info> Withdraw<'info> {
    pub fn withdraw(
        &self,
        amount: u64, // Amount of LP token to burn
        x_min: u64, // Min amount of X we are willing to withdraw
        y_min: u64, // Min amount of Y we are willing to withdraw
        expiration: i64,
    ) -> Result<()> {
        assert_not_locked!(self.config.locked);
        assert_not_expired!(expiration);
        assert_non_zero!([amount]);

        let amounts = ConstantProduct::calculate_withdraw_amounts(
            self.x_vault.amount,
            self.y_vault.amount,
            self.lp_mint.supply,
            amount,
            6
        ).map_err(AmmError::from)?;

        // Check for slippage. As long the user wants to withdraw more than the min
        require!(x_min <= amounts.token_x && y_min <= amounts.token_y, AmmError::SlippageExceeded);
        
        // As usual, we do the trick to try to remove in both
        self.withdraw_tokens(true, amounts.token_x)?;
        self.withdraw_tokens(false, amounts.token_y)?;

        // And we burn the lp tokens 
        self.burn_lp_tokens(amount)
    }

    pub fn withdraw_tokens(
        &self,
        is_x: bool,
        amount:u64,
    ) -> Result<()> {
        // To withdrawal we need to decide who is the from and to
        let (from, to) = match is_x {
            true => (self.x_vault.to_account_info(), self.x_user_ata.to_account_info()),
            false => (self.y_vault.to_account_info(), self.y_user_ata.to_account_info())
        };

        // Define the transfer accounts 
        let cpi_accounts = Transfer {
            from,
            to,
            authority: self.auth.to_account_info(),
        };
        
        // PRepare the seed array (the program will use the auth PDA to sign it)
        let seeds = &[
            &b"auth"[..],
            &[self.config.auth_bump],
        ];

        let signer_seeds = &[&seeds[..]];

        // We define the CPI context
        let ctx = CpiContext::new_with_signer(
            self.token_program.to_account_info(), 
            cpi_accounts,
            signer_seeds
        );

        // We send the transaction
        transfer(ctx, amount)
    }

    pub fn burn_lp_tokens(
        &self,
        amount:u64
    ) -> Result<()> {      
        // Similar to withdraw, burn will require the ctx accounts  
        let cpi_accounts = Burn {
            mint: self.lp_mint.to_account_info(),
            from: self.lp_user_ata.to_account_info(),
            authority: self.payer.to_account_info(),
        };

        // the cpi context
        let ctx = CpiContext::new(
            self.token_program.to_account_info(), 
            cpi_accounts,
        );

        // and send to execute the transaction
        burn(ctx, amount)
    }
}
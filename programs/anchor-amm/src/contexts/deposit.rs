use anchor_lang::prelude::*;
use anchor_spl::{
    associated_token::AssociatedToken,
    token::{mint_to, transfer, MintTo, Token, Transfer},
    token_interface::{Mint, TokenAccount},
};
// use constant_product_curve::ConstantProduct;

use crate::{
    assert_non_zero, assert_not_expired, assert_not_locked, helpers::ConstantProduct, AmmError,
    Config,
};

#[derive(Accounts)]
pub struct Deposit<'info> {
    // as always we add the person who will pay for this instruction (the LP owner)
    #[account(mut)]
    pub payer: Signer<'info>,

    pub x_mint: Box<InterfaceAccount<'info, Mint>>,
    pub y_mint: Box<InterfaceAccount<'info, Mint>>,

    // now we are going to define the lp_mint. IT will contain the token information for our LPs
    #[account(
        init_if_needed,
        payer = payer,
        seeds = [b"lp", config.key().as_ref()],
        bump,
        mint::decimals = 6,
        mint::authority = payer
    )]
    pub lp_mint: Box<InterfaceAccount<'info, Mint>>,

    // We also need the 2 vaults where store X and Y mutables
    #[account(
        mut,
        associated_token::mint = config.x_mint,
        associated_token::authority = auth
    )]
    pub x_vault: Box<InterfaceAccount<'info, TokenAccount>>,
    #[account(
        mut,
        associated_token::mint = config.y_mint,
        associated_token::authority = auth
    )]
    pub y_vault: Box<InterfaceAccount<'info, TokenAccount>>,

    // The two user ATAs which could get changed if the user deposits X or Y
    #[account(
        mut,
        associated_token::mint = config.x_mint,
        associated_token::authority = payer,
    )]
    pub x_user_ata: Box<InterfaceAccount<'info, TokenAccount>>,
    #[account(
        mut,
        associated_token::mint = config.y_mint,
        associated_token::authority = payer,
    )]
    pub y_user_ata: Box<InterfaceAccount<'info, TokenAccount>>,

    // We will need an extra account to save the lp tokens for the user
    #[account(
        init_if_needed,
        payer = payer,
        associated_token::mint = lp_mint,
        associated_token::authority = payer,
    )]
    pub lp_user_ata: Box<InterfaceAccount<'info, TokenAccount>>,

    /// CHECK: this is safe
    #[account(
        seeds = [b"auth"],
        bump = config.auth_bump,
    )]
    pub auth: UncheckedAccount<'info>,

    // We will still need the config account to retrieve some data
    #[account(
        has_one = x_mint,
        has_one = y_mint,
        seeds = [b"config", config.seed.to_le_bytes().as_ref()],
        bump = config.config_bump,
    )]
    pub config: Account<'info, Config>,

    // as always we add the required programs to mint, transfer and create accounts
    pub token_program: Program<'info, Token>,
    pub associated_token_program: Program<'info, AssociatedToken>,
    pub system_program: Program<'info, System>,
}

impl<'info> Deposit<'info> {
    /// Deposit function will verify that the pool is not locked, neither has expired, the amount is not zero
    /// after it will calculate:
    /// - if the pool is empty, it will be able to add the maximum in x and y
    /// - if the pool already has funds, it will  calculate the ratio and multiply/divide to balance the amount added
    pub fn deposit(&mut self, amount: u64, x_max: u64, y_max: u64, expiration: i64) -> Result<()> {
        assert_not_locked!(self.config.locked);
        assert_not_expired!(expiration);
        assert_non_zero!([amount, x_max, y_max]);

        let (x, y) = match self.lp_mint.supply == 0
            && self.x_vault.amount == 0
            && self.y_vault.amount == 0
        {
            true => (x_max, y_max),
            false => {
                let amounts = ConstantProduct::calculate_deposit_amounts(
                    self.x_vault.amount,
                    self.y_vault.amount,
                    self.lp_mint.supply,
                    amount,
                    6,
                )
                .map_err(AmmError::from)?;
                (amounts.token_x, amounts.token_y)
            }
        };

        // The amount of tokens we want to deposit can't exceeded the maximum of tokens based on the current pool liquidity
        require!(x <= x_max && y <= y_max, AmmError::SlippageExceeded);

        // this is a weird way to deposity in X or in Y
        self.deposit_tokens(true, x)?;
        self.deposit_tokens(false, y)?;

        // BAsed on how many tokens the user has deposit, it will get some LP tokens
        self.mint_lp_tokens(amount)?;

        Ok(())
    }

    /// Deposit Tokens
    ///
    /// Helper Function which will have a boolean to specify if it's x or y and the amount to deposit
    pub fn deposit_tokens(&self, is_x: bool, amount: u64) -> Result<()> {
        // If X, from will be user ATA x to vault ATA X
        // Otherwise, from user ATA Y to vault ATA Y
        let (from, to) = match is_x {
            true => (
                self.x_user_ata.to_account_info(),
                self.x_vault.to_account_info(),
            ),
            false => (
                self.y_user_ata.to_account_info(),
                self.y_vault.to_account_info(),
            ),
        };

        // As any CPI call, we will have the accounts, context and cpi method.
        let cpi_accounts = Transfer {
            from,
            to,
            authority: self.payer.to_account_info(),
        };

        // CPI Context
        let ctx = CpiContext::new(self.token_program.to_account_info(), cpi_accounts);

        // Token transfer cpi call
        transfer(ctx, amount)
    }

    /// Mint LP Tokens
    ///
    /// Once the tokens have been deposited, the program will mint LP tokens to the user based on the amount
    pub fn mint_lp_tokens(&self, amount: u64) -> Result<()> {
        // CPI Accounts
        let accounts = MintTo {
            mint: self.lp_mint.to_account_info(),
            to: self.lp_user_ata.to_account_info(),
            authority: self.payer.to_account_info(),
        };

        // As the PDA has to sign the transaction, we need to create the seed based on the LP mint seed
        let seeds = &[&b"auth"[..], &[self.config.auth_bump]];
        let signer_seeds = &[&seeds[..]];

        // We define the Context based on accounts and the PDA signer.
        let ctx = CpiContext::new_with_signer(
            self.token_program.to_account_info(),
            accounts,
            signer_seeds,
        );

        // CPI call to mint LP to the user
        mint_to(ctx, amount)
    }
}

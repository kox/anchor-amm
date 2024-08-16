use anchor_lang::prelude::*;
use anchor_spl::{
    associated_token::AssociatedToken,
    token::{transfer, Token, Transfer},
    token_interface::{Mint, TokenAccount},
};

use crate::{
    assert_non_zero, assert_not_expired, assert_not_locked, Config,
    helpers::{ConstantProduct, LiquidityPair},
    errors::AmmError, 
};

#[derive(Accounts)]
pub struct Swap<'info> {
    pub x_mint: Box<InterfaceAccount<'info, Mint>>,
    pub y_mint: Box<InterfaceAccount<'info, Mint>>,

    #[account(mut)]
    pub payer: Signer<'info>,
    
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

    // now we are going to define the lp_mint. IT will contain the token information for our LPs
    #[account(
        seeds = [b"lp", config.key().as_ref()],
        bump = config.lp_bump,
    )]
    pub lp_mint: Box<InterfaceAccount<'info, Mint>>,

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

    #[account(
        mut,
        seeds = [
            b"config", 
            config.seed.to_le_bytes().as_ref()
        ],
        bump = config.config_bump,
    )]
    pub config: Account<'info, Config>,

    /// CHECK: this is safe
    #[account(
        seeds = [b"auth"],
        bump = config.auth_bump,
    )]
    pub auth: UncheckedAccount<'info>,

    // as always we add the required programs to mint, transfer and create accounts
    pub token_program: Program<'info, Token>,
    pub associated_token_program: Program<'info, AssociatedToken>,
    pub system_program: Program<'info, System>,
}

impl<'info> Swap<'info> {
    /// Execute a swap from X to Y or Y to X
    ///
    /// # Arguments
    ///
    /// * `amount_in` - The amount of input tokens (either X or Y) to swap.
    /// * `min_amount_out` - The minimum amount of output tokens the user expects to receive (to enforce slippage protection).
    /// * `is_x_to_y` - Boolean indicating whether the swap is from X to Y (true) or Y to X (false).
    /// * `expiration` - timestamp to restrict old swaps which can be expired
    pub fn swap(
        &mut self,
        amount_in: u64,
        min_amount_out: u64,
        is_x_to_y: bool,
        expiration: i64,
    ) -> Result<()> {
        // Ensure the input amount is non-zero
        assert_non_zero!([amount_in]);
        assert_not_locked!(self.config.locked);
        assert_not_expired!(expiration);

        // Retrieve the current state of the Constant Product curve
        let mut curve = ConstantProduct::init(
            self.x_vault.amount,
            self.y_vault.amount,
            self.lp_mint.supply,
            self.config.fee,
            Some(6), // Assuming 6 decimal precision for calculations
        )
        .map_err(AmmError::from)?;

        let pair = match is_x_to_y {
            true => LiquidityPair::TokenX,
            false => LiquidityPair::TokenY,
        };

        let swap_result = curve
            .swap(pair, amount_in, min_amount_out)
            .map_err(AmmError::from)?;

        assert_non_zero!([swap_result.deposited, swap_result.withdrawn]);

        // Transfer the input tokens from the user to the vault
        self.deposit_tokens(is_x_to_y, swap_result.deposited)?;

        // Transfer the output tokens from the vault to the user
        self.withdraw_tokens(!is_x_to_y, swap_result.withdrawn)?;
 
        Ok(())
    }

    /// Deposit Tokens
    ///
    /// Helper function to deposit tokens (X or Y) to the vault's ATA
    fn deposit_tokens(&self, is_x_to_y: bool, deposited: u64) -> Result<()> {
        let (from, to) = match is_x_to_y {
            true => (
                self.x_user_ata.to_account_info(),
                self.x_vault.to_account_info(),
            ),
            false => (
                self.y_user_ata.to_account_info(),
                self.y_vault.to_account_info(),
            ),
        };

        let cpi_program = self.token_program.to_account_info();

        let cpi_accounts = Transfer {
            from,
            to,
            authority: self.payer.to_account_info(),
        };

        let ctx = CpiContext::new(cpi_program, cpi_accounts);

        transfer(ctx, deposited)
    }

    /// Withdraw Tokens
    ///
    /// Helper function to withdraw tokens (X or Y) to the user's ATA
    fn withdraw_tokens(&mut self, is_x_to_y: bool, withdrawn: u64) -> Result<()> {
        let (from, to) = match is_x_to_y {
            true => (
                self.y_vault.to_account_info(),
                self.y_user_ata.to_account_info(),
            ),
            false => (
                self.x_vault.to_account_info(),
                self.x_user_ata.to_account_info(),
            ),
        };

        let cpi_program = self.token_program.to_account_info();

        let seeds = &[&b"auth"[..], &[self.config.auth_bump]];

        let signer_seeds = &[&seeds[..]];

        let cpi_accounts = Transfer {
            from,
            to,
            authority: self.auth.to_account_info(),
        };

        let ctx = CpiContext::new_with_signer(cpi_program, cpi_accounts, signer_seeds);

        transfer(ctx, withdrawn)
    }
}

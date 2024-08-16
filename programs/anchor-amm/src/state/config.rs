use anchor_lang::prelude::*;

use crate::{BOOL_L, OPTION_L, PUBKEY_L, U16_L, U64_L, U8_L};



/// Config struct will save most of the important information for the LP
#[account]
pub struct Config {
    // Random number to make it unique
    pub seed: u64,
    // Optioanl public key which will have the right to change the configuration 
    pub authority: Option<Pubkey>,
    // Public keys for the X and Y accounts containing relevant information of the Token Account
    pub x_mint: Pubkey,
    pub y_mint: Pubkey,
    // How much is going to cost to the users to utilize this LP
    pub fee: u16,
    // Variable to allow or lock the LP  
    pub locked: bool,
    // We save the bumps to perform better the PDA seed discovery 
    pub auth_bump: u8,
    pub config_bump: u8,
    pub lp_bump: u8,
}

impl Config {
    pub const INIT_SPACE: usize = 8 + U64_L + OPTION_L + PUBKEY_L*3 + U16_L + BOOL_L + U8_L*3;

    pub fn init(
        &mut self,
        seed: u64,
        authority: Option<Pubkey>,
        x_mint: Pubkey,
        y_mint: Pubkey,
        fee: u16,
        auth_bump: u8,
        config_bump: u8,
        lp_bump: u8,
    ) {
        self.seed = seed;
        self.authority = authority;
        self.x_mint = x_mint;
        self.y_mint = y_mint;
        self.fee = fee;
        self.locked = false;
        self.auth_bump = auth_bump;
        self.config_bump = config_bump; 
        self.lp_bump = lp_bump; 
    }
} 
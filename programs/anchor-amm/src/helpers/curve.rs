use std::error::Error;
use std::fmt;

// Macro to assert that no elements in the array are zero.
macro_rules! assert_non_zero {
    ($array:expr) => {
        if $array.contains(&0u64) {
            return Err(CurveError::ZeroBalance)
        }
    };
}

// Macro to enforce slippage limits on withdrawals.
macro_rules! withdraw_slippage {
    ($amount_x:expr, $amount_y:expr, $min_x:expr, $min_y:expr) => {
        if $amount_x < $min_x || $amount_y < $min_y {
            return Err(CurveError::SlippageLimitExceeded)
        }
    };
}

// Macro to enforce slippage limits on deposits.
macro_rules! deposit_slippage {
    ($amount_x:expr, $amount_y:expr, $max_x:expr, $max_y:expr) => {
        if $amount_x > $max_x || $amount_y > $max_y {
            return Err(CurveError::SlippageLimitExceeded)
        }
    };
}

// Macro to enforce slippage limits on swaps.
macro_rules! swap_slippage {
    ($amount:expr, $min_amount:expr) => {
        if $amount < $min_amount {
            return Err(CurveError::SlippageLimitExceeded)
        }
    };
}

// Enum to represent the token pair being swapped.
#[derive(Debug)]
pub enum LiquidityPair {
    TokenX,
    TokenY,
}

// Struct to represent the spot price of a token.
#[derive(Debug)]
pub struct SpotPrice {
    pub amount: u128,  // The spot price in terms of the other token.
    pub precision: u32, // The precision for representing the spot price.
}

// Struct to represent amounts of tokens X and Y.
#[derive(Debug)]
pub struct TokenAmounts {
    pub token_x: u64,
    pub token_y: u64,
}

// Struct to represent the result of a liquidity deposit operation.
#[derive(Debug)]
pub struct DepositLiquidityResult {
    pub deposited_x: u64,  // Amount of token X deposited.
    pub deposited_y: u64,  // Amount of token Y deposited.
    pub minted_lp_tokens: u64,  // Amount of LP tokens minted as a result.
}

// Struct to represent the result of a liquidity withdrawal operation.
#[derive(Debug)]
pub struct WithdrawLiquidityResult {
    pub withdrawn_x: u64,  // Amount of token X withdrawn.
    pub withdrawn_y: u64,  // Amount of token Y withdrawn.
    pub burned_lp_tokens: u64,  // Amount of LP tokens burned as a result.
}

// Struct to represent the result of a swap operation.
#[derive(Debug)]
pub struct SwapResult {
    pub deposited: u64,  // Amount of the input token deposited.
    pub withdrawn: u64,  // Amount of the output token withdrawn.
    pub fee: u64,  // Fee taken for the swap operation.
}

// Enum to represent various errors that might occur in the curve operations.
#[derive(Debug)]
pub enum CurveError {
    InvalidPrecision,  // Error when precision is invalid.
    Overflow,  // Error when an arithmetic overflow occurs.
    Underflow,  // Error when an arithmetic underflow occurs.
    InvalidFeeAmount,  // Error when the fee amount is invalid.
    InsufficientBalance,  // Error when there's an insufficient balance.
    ZeroBalance,  // Error when one of the balances is zero.
    SlippageLimitExceeded,  // Error when the slippage limit is exceeded.
}

impl Error for CurveError {}

impl fmt::Display for CurveError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}

// Struct representing the Constant Product AMM curve.
#[derive(Debug)]
pub struct ConstantProduct {
    balance_x: u64,  // Balance of Token X in the pool.
    balance_y: u64,  // Balance of Token Y in the pool.
    total_lp_tokens: u64,  // Total LP tokens issued for this pool.
    fee_basis_points: u16,  // Fee taken for each operation, in basis points (1% = 100 basis points).
    precision: u32,  // Precision used for calculations to avoid rounding errors.
}

// Main Implementation of the ConstantProduct struct
impl ConstantProduct {

    // Initialize a new Constant Product curve.
    pub fn init(balance_x: u64, balance_y: u64, initial_lp_tokens: u64, fee_basis_points: u16, precision: Option<u8>) -> Result<ConstantProduct, CurveError> {
        // Assert non-zero values for X and Y balances.
        assert_non_zero!([balance_x, balance_y]);

        // Set precision, defaulting to 1,000,000 if not provided.
        let precision = match precision {
            Some(p) => 10u32.checked_pow(p as u32).ok_or(CurveError::InvalidPrecision)?,
            None => 1_000_000,
        };
        
        // If no initial LP tokens are provided, set it to the maximum of X or Y to minimize rounding errors.
        let total_lp_tokens = if initial_lp_tokens > 0 {
            initial_lp_tokens
        } else {
            balance_x.max(balance_y)
        };

        Ok(ConstantProduct {
            balance_x,
            balance_y,
            total_lp_tokens,
            fee_basis_points,
            precision,
        })
    }

    ////////////////////
    // Static methods //
    ////////////////////

    // Calculate the invariant (K) for the pool, K = X * Y
    pub fn calculate_invariant(balance_x: u64, balance_y: u64) -> Result<u128, CurveError> {
        assert_non_zero!([balance_x, balance_y]);
        Ok((balance_x as u128).checked_mul(balance_y as u128).ok_or(CurveError::Overflow)?)
    }

    // Calculate the spot price of Token X in terms of Token Y.
    pub fn calculate_spot_price_x(balance_x: u64, balance_y: u64, precision: u32) -> Result<SpotPrice, CurveError> {
        assert_non_zero!([balance_x, balance_y]);
        Ok(
            SpotPrice {
                amount: (balance_x as u128)
                    .checked_mul(precision as u128).ok_or(CurveError::Overflow)?
                    .checked_div(balance_y as u128).ok_or(CurveError::Overflow)?, 
                precision
            }
        )
    }

    // Calculate the spot price of Token Y in terms of Token X.
    pub fn calculate_spot_price_y(balance_x: u64, balance_y: u64, precision: u32) -> Result<SpotPrice, CurveError> {
        Self::calculate_spot_price_x(balance_y, balance_x, precision)
    }

    // Calculate the amount of X and Y required to deposit a specific amount of LP tokens.
    pub fn calculate_deposit_amounts(balance_x: u64, balance_y: u64, total_lp_tokens: u64, lp_tokens_to_mint: u64, precision: u32) -> Result<TokenAmounts, CurveError> {
        let ratio = (total_lp_tokens as u128)
            .checked_add(lp_tokens_to_mint as u128).ok_or(CurveError::Overflow)?
            .checked_mul(precision as u128).ok_or(CurveError::Overflow)?
            .checked_div(total_lp_tokens as u128).ok_or(CurveError::Overflow)?;

        let deposit_x = (balance_x as u128)
            .checked_mul(ratio).ok_or(CurveError::Overflow)?
            .checked_div(precision as u128).ok_or(CurveError::Overflow)?
            .checked_sub(balance_x as u128).ok_or(CurveError::Overflow)? as u64;

        let deposit_y = (balance_y as u128)
            .checked_mul(ratio).ok_or(CurveError::Overflow)?
            .checked_div(precision as u128).ok_or(CurveError::Overflow)?
            .checked_sub(balance_y as u128).ok_or(CurveError::Overflow)? as u64;

        Ok(TokenAmounts {
            token_x: deposit_x,
            token_y: deposit_y,
        })
    }

    // Calculate the amount of X and Y that will be withdrawn when burning LP tokens.
    pub fn calculate_withdraw_amounts(balance_x: u64, balance_y: u64, total_lp_tokens: u64, lp_tokens_to_burn: u64, precision: u32) -> Result<TokenAmounts, CurveError> {
        let ratio = ((total_lp_tokens - lp_tokens_to_burn) as u128)
            .checked_mul(precision as u128).ok_or(CurveError::Overflow)?
            .checked_div(total_lp_tokens as u128).ok_or(CurveError::Overflow)?;

        let withdraw_x = (balance_x as u128)
            .checked_sub((balance_x as u128)
                .checked_mul(ratio).ok_or(CurveError::Overflow)?
                .checked_div(precision as u128).ok_or(CurveError::Overflow)?
            ).ok_or(CurveError::Overflow)? as u64;

        let withdraw_y = (balance_y as u128)
            .checked_sub((balance_y as u128)
                .checked_mul(ratio).ok_or(CurveError::Overflow)?
                .checked_div(precision as u128).ok_or(CurveError::Overflow)?
            ).ok_or(CurveError::Overflow)? as u64;

        Ok(TokenAmounts {
            token_x: withdraw_x, 
            token_y: withdraw_y,
        })
    }

    // Calculate the new value of X after depositing a specific amount of Y in a swap.
    pub fn calculate_new_x_after_y_swap(balance_x: u64, balance_y: u64, amount_y: u64) -> Result<u64, CurveError> {
        let invariant = Self::calculate_invariant(balance_x, balance_y)?;
        let new_y = (balance_y as u128).checked_add(amount_y as u128).ok_or(CurveError::Overflow)?;
        Ok(invariant.checked_div(new_y).ok_or(CurveError::Overflow)? as u64)
    }

    // Calculate the new value of Y after depositing a specific amount of X in a swap.
    pub fn calculate_new_y_after_x_swap(balance_x: u64, balance_y: u64, amount_x: u64) -> Result<u64, CurveError> {
        Self::calculate_new_x_after_y_swap(balance_y, balance_x, amount_x)
    }

    // Calculate the difference in X from swapping in Y.
    pub fn calculate_x_difference_from_y_swap(balance_x: u64, balance_y: u64, amount_y: u64) -> Result<u64, CurveError> {
        Ok(balance_x.checked_sub(Self::calculate_new_x_after_y_swap(balance_x, balance_y, amount_y)?).ok_or(CurveError::Overflow)?)
    }

    // Calculate the difference in Y from swapping in X.
    pub fn calculate_y_difference_from_x_swap(balance_x: u64, balance_y: u64, amount_x: u64) -> Result<u64, CurveError> {
        Self::calculate_x_difference_from_y_swap(balance_y, balance_x, amount_x)
    }

    ////////////////////
    // Getter methods //
    ////////////////////

    // Calculate the current invariant (K) value, K = X * Y.
    pub fn get_invariant(&self) -> Result<u128, CurveError> {
        Self::calculate_invariant(self.balance_x, self.balance_y)
    }

    // Get the spot price of Token X in terms of Token Y.
    pub fn get_spot_price_x(&self) -> Result<SpotPrice, CurveError> {
        Self::calculate_spot_price_x(self.balance_x, self.balance_y, self.precision)
    }

    // Get the spot price of Token Y in terms of Token X.
    pub fn get_spot_price_y(&self) -> Result<SpotPrice, CurveError> {
        Self::calculate_spot_price_y(self.balance_x, self.balance_y, self.precision)
    }

    ////////////////////
    // Setter methods //
    ////////////////////

    // Unsafe method to swap Token X for Token Y or vice versa without slippage protection.
    pub fn swap_unsafe(&mut self, token_pair: LiquidityPair, amount: u64) -> Result<SwapResult, CurveError> {
        // Calculate the effective amount after deducting the fee.
        let effective_amount = (amount as u128)
            .checked_mul((10_000 - self.fee_basis_points) as u128).ok_or(CurveError::Overflow)?
            .checked_div(10_000).ok_or(CurveError::Overflow)? as u64;

        // Depending on the token pair, calculate the new balances and the amount to withdraw.
        let (new_x, new_y, withdrawn_amount) = match token_pair {
            LiquidityPair::TokenX => {
                (
                    self.balance_x.checked_add(effective_amount).ok_or(CurveError::Overflow)?,
                    Self::calculate_new_y_after_x_swap(self.balance_x, self.balance_y, effective_amount)?,
                    Self::calculate_y_difference_from_x_swap(self.balance_x, self.balance_y, effective_amount)?,
                )
            },
            LiquidityPair::TokenY => {
                (
                    Self::calculate_new_x_after_y_swap(self.balance_x, self.balance_y, amount)?,
                    self.balance_y.checked_add(amount).ok_or(CurveError::Overflow)?,
                    Self::calculate_x_difference_from_y_swap(self.balance_x, self.balance_y, effective_amount)?,
                )
            }
        };

        // Calculate the fee.
        let fee = amount.checked_sub(effective_amount).ok_or(CurveError::Underflow)?;

        // Update balances.
        self.balance_x = new_x;
        self.balance_y = new_y;

        Ok(SwapResult {
            deposited: amount,
            fee,
            withdrawn: withdrawn_amount,
        })
    }

    // Swap tokens with slippage protection.
    pub fn swap(&mut self, token_pair: LiquidityPair, amount: u64, min_withdrawn: u64) -> Result<SwapResult, CurveError> {
        // Calculate the effective amount after deducting the fee.
        let effective_amount = (amount as u128)
            .checked_mul((10_000 - self.fee_basis_points) as u128)
            .ok_or(CurveError::Overflow)?
            .checked_div(10_000)
            .ok_or(CurveError::Overflow)? as u64;
    
        // Depending on the token pair, calculate the new balances and the amount to withdraw.
        let (new_x, new_y, withdrawn_amount) = match token_pair {
            LiquidityPair::TokenX => {
                let new_x = self.balance_x.checked_add(effective_amount).ok_or(CurveError::Overflow)?;
                let new_y = Self::calculate_new_y_after_x_swap(self.balance_x, self.balance_y, effective_amount)?;
                let delta_y = Self::calculate_y_difference_from_x_swap(self.balance_x, self.balance_y, effective_amount)?;
                (new_x, new_y, delta_y)
            }
            LiquidityPair::TokenY => {
                let new_x = Self::calculate_new_x_after_y_swap(self.balance_x, self.balance_y, amount)?;
                let new_y = self.balance_y.checked_add(amount).ok_or(CurveError::Overflow)?;
                let delta_x = Self::calculate_x_difference_from_y_swap(self.balance_x, self.balance_y, effective_amount)?;
                (new_x, new_y, delta_x)
            }
        };
    
        // Ensure that the withdrawn amount meets the minimum slippage requirement.
        swap_slippage!(withdrawn_amount, min_withdrawn);

        // Calculate the fee.
        let fee = amount.checked_sub(effective_amount).ok_or(CurveError::Underflow)?;

        // Update balances.
        self.balance_x = new_x;
        self.balance_y = new_y;
    
        Ok(SwapResult {
            deposited: amount,
            fee,
            withdrawn: withdrawn_amount,
        })
    }

    // Unsafe method to deposit liquidity without slippage protection.
    pub fn deposit_liquidity_unsafe(&mut self, amount_x: u64, amount_y: u64, lp_tokens_to_mint: u64) -> Result<DepositLiquidityResult, CurveError> {
        self.balance_x.checked_add(amount_x).ok_or(CurveError::Overflow)?;
        self.balance_y.checked_add(amount_y).ok_or(CurveError::Overflow)?;
        self.total_lp_tokens.checked_add(lp_tokens_to_mint).ok_or(CurveError::Overflow)?;
        Ok(DepositLiquidityResult { deposited_x: amount_x, deposited_y: amount_y, minted_lp_tokens: lp_tokens_to_mint })
    }

    // Unsafe method to withdraw liquidity without slippage protection.
    pub fn withdraw_liquidity_unsafe(&mut self, amount_x: u64, amount_y: u64, lp_tokens_to_burn: u64) -> Result<WithdrawLiquidityResult, CurveError> {
        self.balance_x.checked_sub(amount_x).ok_or(CurveError::Underflow)?;
        self.balance_y.checked_sub(amount_y).ok_or(CurveError::Underflow)?;
        self.total_lp_tokens.checked_sub(lp_tokens_to_burn).ok_or(CurveError::Underflow)?;
        Ok(WithdrawLiquidityResult { withdrawn_x: amount_x, withdrawn_y: amount_y, burned_lp_tokens: lp_tokens_to_burn })
    }

    // Deposit liquidity into the pool with slippage protection.
    pub fn deposit_liquidity(&mut self, lp_tokens_to_mint: u64, max_x: u64, max_y: u64) -> Result<DepositLiquidityResult, CurveError> {
        let amounts = Self::calculate_deposit_amounts(self.balance_x, self.balance_y, self.total_lp_tokens, lp_tokens_to_mint, self.precision)?;
        deposit_slippage!(amounts.token_x, amounts.token_y, max_x, max_y);
        self.deposit_liquidity_unsafe(amounts.token_x, amounts.token_y, lp_tokens_to_mint)
    }

    // Withdraw liquidity from the pool with slippage protection.
    pub fn withdraw_liquidity(&mut self, lp_tokens_to_burn: u64, min_x: u64, min_y: u64) -> Result<WithdrawLiquidityResult, CurveError> {
        let amounts = Self::calculate_withdraw_amounts(self.balance_x, self.balance_y, self.total_lp_tokens, lp_tokens_to_burn, self.precision)?;
        withdraw_slippage!(amounts.token_x, amounts.token_y, min_x, min_y);  
        self.withdraw_liquidity_unsafe(amounts.token_x, amounts.token_y, lp_tokens_to_burn)
    }
}

#[cfg(test)]
mod tests {
    use crate::helpers::{ConstantProduct, LiquidityPair};

    #[test]
    fn swap_balance() {
        // If we start with 20 of token X and 30 of token Y and precision of 1, K should equal 600
        let mut pool = ConstantProduct::init(20, 30, 0, 0, None).unwrap();
        assert_eq!(pool.get_invariant().unwrap(), 600);
        assert_eq!(pool.balance_x, 20);
        assert_eq!(pool.balance_y, 30);

        // If we deposit 5 of token X, the user should receive 6 of token Y.
        // The final balances should be - Token X: 25, Token Y: 24.
        let res = pool.swap(LiquidityPair::TokenX, 5, 6).unwrap();
        assert_eq!(pool.get_invariant().unwrap(), 600);
        assert_eq!(res.deposited, 5);
        assert_eq!(res.withdrawn, 6);
        assert_eq!(pool.balance_x, 25);
        assert_eq!(pool.balance_y, 24);

        // If we deposit another 5 of token X, the user should receive 4 of token Y.
        // The final balances should be - Token X: 30, Token Y: 20.
        let res = pool.swap(LiquidityPair::TokenX, 5, 4).unwrap();
        assert_eq!(pool.get_invariant().unwrap(), 600);
        assert_eq!(res.deposited, 5);
        assert_eq!(res.withdrawn, 4);
        assert_eq!(pool.balance_x, 30);
        assert_eq!(pool.balance_y, 20);
    }

    #[test]
    fn swap_balance_reverse() {
        // Prove this also works in reverse, if we start with 20 of token X and 30 of token Y and precision of 1, K should equal 600
        let mut pool = ConstantProduct::init(30, 20, 0, 0, None).unwrap();
        assert_eq!(pool.get_invariant().unwrap(), 600);
        assert_eq!(pool.balance_x, 30);
        assert_eq!(pool.balance_y, 20);

        // If we deposit 5 of token Y, the user should receive 6 of token X.
        // The final balances should be - Token Y: 25, Token X: 24.
        let res = pool.swap(LiquidityPair::TokenY, 5, 6).unwrap();
        assert_eq!(pool.get_invariant().unwrap(), 600);
        assert_eq!(res.deposited, 5);
        assert_eq!(res.withdrawn, 6);
        assert_eq!(pool.balance_x, 24);
        assert_eq!(pool.balance_y, 25);

        // If we deposit another 5 of token Y, the user should receive 4 of token X.
        // The final balances should be - Token Y: 30, Token X: 20.
        let res = pool.swap(LiquidityPair::TokenY, 5, 4).unwrap();
        assert_eq!(res.deposited, 5);
        assert_eq!(res.withdrawn, 4);
        assert_eq!(pool.balance_x, 20);
        assert_eq!(pool.balance_y, 30);
    }

    #[test]
    fn swap_balance_with_fee() {
        // If we start with 20 of token X and 30 of token Y and precision of 1, K should equal 600
        let mut pool = ConstantProduct::init(20, 30, 0, 100, None).unwrap();
        assert_eq!(pool.get_invariant().unwrap(), 600);
        assert_eq!(pool.balance_x, 20);
        assert_eq!(pool.balance_y, 30);

        // If we deposit 5 of token X, the user should pay a fee of 1 and thus only receive 5 of token Y.
        // The final balances should be - Token X: 24, Token Y: 25.
        let res = pool.swap(LiquidityPair::TokenX, 5, 5).unwrap();
        assert_eq!(res.deposited, 5);
        assert_eq!(res.withdrawn, 5);
        assert_eq!(res.fee, 1);
        assert_eq!(pool.balance_x, 24);
        assert_eq!(pool.balance_y, 25);
    }

    #[test]
    fn deposit_liquidity() {
        // If we start with 30 of token X and 30 of token Y and precision of 1, K should equal 900
        let mut pool = ConstantProduct::init(30, 30, 0, 100, None).unwrap();
        assert_eq!(pool.get_invariant().unwrap(), 900);
        assert_eq!(pool.balance_x, 30);
        assert_eq!(pool.balance_y, 30);

        // Deposit 30 LP tokens and assert balances.
        let r = pool.deposit_liquidity(30, 10000000, 10000000).unwrap();
        assert_eq!(r.deposited_x, 30);
        assert_eq!(r.deposited_y, 30);
        assert_eq!(r.minted_lp_tokens, 30);

        // Withdraw 30 LP tokens and assert balances.
        let r = pool.withdraw_liquidity(30, 0, 0).unwrap();
        assert_eq!(r.withdrawn_x, 30);
        assert_eq!(r.withdrawn_y, 30);
        assert_eq!(r.burned_lp_tokens, 30);
    }

    #[test]
    fn spot_price() {
        let pool = ConstantProduct::init(10, 10, 0, 100, Some(0)).unwrap();
        assert_eq!(pool.get_spot_price_x().unwrap().amount, pool.get_spot_price_y().unwrap().amount);
        assert_eq!(pool.get_spot_price_x().unwrap().amount, 1)
    }
}

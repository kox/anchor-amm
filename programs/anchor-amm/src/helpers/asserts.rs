/// List of macros which act as helperss
/// 
/// Macro as useful reserved words to inject some rust code inside of functions
/// 

/// assert_non_zero
/// 
/// Macro which check if an array contains a number u64. If the array is empty, it will return an error
#[macro_export]
macro_rules! assert_non_zero {
    ($array:expr) => {
        if $array.contains(&0u64) {
            return err!(AmmError::ZeroBalance)
        }
    };
}

/// assert_not_locked
/// 
/// Macro which checks if lock variable is false. If true, it will return an error
#[macro_export]
macro_rules! assert_not_locked {
    ($lock:expr) => {
        if $lock == true {
            return err!(AmmError::PoolLocked)
        }
    };
}

/// assert_not_expired
/// 
/// Macro to check if the expiration time has expired. It uses unix_timestamp 
/// (possible better option is using slots)
#[macro_export]
macro_rules! assert_not_expired {
    ($expiration:expr) => {
        if Clock::get()?.unix_timestamp > $expiration {
            return err!(AmmError::OfferExpired);
        }
    };
}

/// has_update_authority
/// 
/// Macro to verify if the LP config has authority setup (it's optional)
/// If yes, it will verify that the user trying to do the action, it's the same than the config one
/// If no, it will return an error because it's not possible to change the config data after creation
#[macro_export]
macro_rules! has_update_authority {
    ($x:expr) => {
        match $x.config.authority {
            Some(a) => {
                require_keys_eq!(a, $x.user.key(), AmmError::InvalidAuthority);
            },
            None => return err!(AmmError::NoAuthoritySet)
        }
    };
}


use solana_program::{
    program_error::ProgramError,
    pubkey::Pubkey,
};

use crate::error::VaultError;

/// Seeds for vault state PDA derivation
pub const VAULT_SEED: &[u8] = b"vault";

/// Seeds for user balance PDA derivation
pub const USER_BALANCE_SEED: &[u8] = b"user_balance";

/// Derive vault state PDA from owner and token mint
pub fn derive_vault_state_pda(
    program_id: &Pubkey,
    owner: &Pubkey,
    token_mint: &Pubkey,
) -> Result<(Pubkey, u8), ProgramError> {
    let seeds = &[
        VAULT_SEED,
        owner.as_ref(),
        token_mint.as_ref(),
    ];
    
    Ok(Pubkey::find_program_address(seeds, program_id))
}

/// Derive user balance PDA from user and vault state
pub fn derive_user_balance_pda(
    program_id: &Pubkey,
    user: &Pubkey,
    vault_state: &Pubkey,
) -> Result<(Pubkey, u8), ProgramError> {
    let seeds = &[
        USER_BALANCE_SEED,
        user.as_ref(),
        vault_state.as_ref(),
    ];
    
    Ok(Pubkey::find_program_address(seeds, program_id))
}

/// Verify vault state PDA derivation
pub fn verify_vault_state_pda(
    program_id: &Pubkey,
    vault_state: &Pubkey,
    owner: &Pubkey,
    token_mint: &Pubkey,
    bump: u8,
) -> Result<(), ProgramError> {
    let seeds = &[
        VAULT_SEED,
        owner.as_ref(),
        token_mint.as_ref(),
        &[bump],
    ];
    
    let expected_pda = Pubkey::create_program_address(seeds, program_id)
        .map_err(|_| VaultError::InvalidInput)?;
    
    if expected_pda != *vault_state {
        return Err(VaultError::InvalidInput.into());
    }
    
    Ok(())
}

/// Verify user balance PDA derivation
pub fn verify_user_balance_pda(
    program_id: &Pubkey,
    user_balance: &Pubkey,
    user: &Pubkey,
    vault_state: &Pubkey,
    bump: u8,
) -> Result<(), ProgramError> {
    let seeds = &[
        USER_BALANCE_SEED,
        user.as_ref(),
        vault_state.as_ref(),
        &[bump],
    ];
    
    let expected_pda = Pubkey::create_program_address(seeds, program_id)
        .map_err(|_| VaultError::InvalidInput)?;
    
    if expected_pda != *user_balance {
        return Err(VaultError::InvalidInput.into());
    }
    
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use solana_program::system_program;

    #[test]
    fn test_vault_state_pda_derivation() {
        let program_id = system_program::id(); // Using system program ID for testing
        let owner = Pubkey::new_unique();
        let token_mint = Pubkey::new_unique();
        
        let (pda, bump) = derive_vault_state_pda(&program_id, &owner, &token_mint).unwrap();
        
        // Verify the PDA can be recreated with the same inputs
        let verification = verify_vault_state_pda(&program_id, &pda, &owner, &token_mint, bump);
        assert!(verification.is_ok());
    }

    #[test]
    fn test_user_balance_pda_derivation() {
        let program_id = system_program::id(); // Using system program ID for testing
        let user = Pubkey::new_unique();
        let vault_state = Pubkey::new_unique();
        
        let (pda, bump) = derive_user_balance_pda(&program_id, &user, &vault_state).unwrap();
        
        // Verify the PDA can be recreated with the same inputs
        let verification = verify_user_balance_pda(&program_id, &pda, &user, &vault_state, bump);
        assert!(verification.is_ok());
    }

    #[test]
    fn test_invalid_vault_state_pda_verification() {
        let program_id = system_program::id();
        let owner = Pubkey::new_unique();
        let token_mint = Pubkey::new_unique();
        let wrong_pda = Pubkey::new_unique();
        
        let verification = verify_vault_state_pda(&program_id, &wrong_pda, &owner, &token_mint, 255);
        assert!(verification.is_err());
    }

    #[test]
    fn test_invalid_user_balance_pda_verification() {
        let program_id = system_program::id();
        let user = Pubkey::new_unique();
        let vault_state = Pubkey::new_unique();
        let wrong_pda = Pubkey::new_unique();
        
        let verification = verify_user_balance_pda(&program_id, &wrong_pda, &user, &vault_state, 255);
        assert!(verification.is_err());
    }
}

/// Account validation utilities
use solana_program::{
    account_info::AccountInfo,
    program_pack::Pack,
    system_program,
};

/// Verify that an account is a signer
pub fn verify_signer(account: &AccountInfo) -> Result<(), ProgramError> {
    if !account.is_signer {
        return Err(VaultError::UnauthorizedAccess.into());
    }
    Ok(())
}

/// Verify that an account is writable
pub fn verify_writable(account: &AccountInfo) -> Result<(), ProgramError> {
    if !account.is_writable {
        return Err(VaultError::InvalidInput.into());
    }
    Ok(())
}

/// Verify that an account is owned by the expected program
pub fn verify_account_owner(
    account: &AccountInfo,
    expected_owner: &Pubkey,
) -> Result<(), ProgramError> {
    if account.owner != expected_owner {
        return Err(VaultError::InvalidTokenAccount.into());
    }
    Ok(())
}

/// Verify that an account is a valid SPL token account
pub fn verify_token_account(
    account: &AccountInfo,
    expected_mint: Option<&Pubkey>,
) -> Result<(), ProgramError> {
    // Check that the account is owned by the SPL Token program
    verify_account_owner(account, &spl_token::id())?;
    
    // If expected mint is provided, verify it matches
    if let Some(expected_mint) = expected_mint {
        let token_account = spl_token::state::Account::unpack(&account.data.borrow())
            .map_err(|_| VaultError::InvalidTokenAccount)?;
        
        if token_account.mint != *expected_mint {
            return Err(VaultError::InvalidMint.into());
        }
    }
    
    Ok(())
}

/// Verify that an account is a valid SPL token mint
pub fn verify_token_mint(account: &AccountInfo) -> Result<(), ProgramError> {
    // Check that the account is owned by the SPL Token program
    verify_account_owner(account, &spl_token::id())?;
    
    // Try to unpack as a mint to verify structure
    spl_token::state::Mint::unpack(&account.data.borrow())
        .map_err(|_| VaultError::InvalidMint)?;
    
    Ok(())
}

/// Verify that an account is uninitialized (for PDA creation)
pub fn verify_uninitialized_account(account: &AccountInfo) -> Result<(), ProgramError> {
    if account.owner != &system_program::id() {
        return Err(VaultError::AccountNotInitialized.into());
    }
    
    if account.data_len() != 0 {
        return Err(VaultError::AccountNotInitialized.into());
    }
    
    Ok(())
}

/// Verify that an account has sufficient lamports for rent exemption
pub fn verify_rent_exempt(
    account: &AccountInfo,
    rent: &solana_program::sysvar::rent::Rent,
) -> Result<(), ProgramError> {
    if !rent.is_exempt(account.lamports(), account.data_len()) {
        return Err(VaultError::InvalidInput.into());
    }
    Ok(())
}

/// Comprehensive account validation for vault operations
pub fn validate_vault_accounts(
    owner: &AccountInfo,
    vault_state: &AccountInfo,
    vault_token_account: &AccountInfo,
    token_mint: &AccountInfo,
    program_id: &Pubkey,
) -> Result<(), ProgramError> {
    // Verify owner is signer
    verify_signer(owner)?;
    
    // Verify vault state account is owned by our program
    verify_account_owner(vault_state, program_id)?;
    
    // Verify vault token account is valid SPL token account
    verify_token_account(vault_token_account, Some(token_mint.key))?;
    
    // Verify token mint is valid
    verify_token_mint(token_mint)?;
    
    Ok(())
}

/// Validate user operation accounts
pub fn validate_user_accounts(
    user: &AccountInfo,
    user_token_account: &AccountInfo,
    vault_token_account: &AccountInfo,
    vault_state: &AccountInfo,
    user_balance: &AccountInfo,
    program_id: &Pubkey,
    token_mint: &Pubkey,
) -> Result<(), ProgramError> {
    // Verify user is signer
    verify_signer(user)?;
    
    // Verify user token account is valid and matches mint
    verify_token_account(user_token_account, Some(token_mint))?;
    
    // Verify vault token account is valid and matches mint
    verify_token_account(vault_token_account, Some(token_mint))?;
    
    // Verify vault state account is owned by our program
    verify_account_owner(vault_state, program_id)?;
    
    // Verify user balance account is owned by our program
    verify_account_owner(user_balance, program_id)?;
    
    Ok(())
}

#[cfg(test)]
mod validation_tests {
    use super::*;
    use solana_program::clock::Epoch;

    fn create_test_account_info<'a>(
        key: &'a Pubkey,
        is_signer: bool,
        is_writable: bool,
        lamports: &'a mut u64,
        data: &'a mut [u8],
        owner: &'a Pubkey,
    ) -> AccountInfo<'a> {
        AccountInfo {
            key,
            is_signer,
            is_writable,
            lamports: std::rc::Rc::new(std::cell::RefCell::new(lamports)),
            data: std::rc::Rc::new(std::cell::RefCell::new(data)),
            owner,
            executable: false,
            rent_epoch: Epoch::default(),
        }
    }

    #[test]
    fn test_verify_signer_success() {
        let key = Pubkey::new_unique();
        let mut lamports = 0;
        let mut data = [];
        let owner = system_program::id();
        
        let account = create_test_account_info(&key, true, false, &mut lamports, &mut data, &owner);
        assert!(verify_signer(&account).is_ok());
    }

    #[test]
    fn test_verify_signer_failure() {
        let key = Pubkey::new_unique();
        let mut lamports = 0;
        let mut data = [];
        let owner = system_program::id();
        
        let account = create_test_account_info(&key, false, false, &mut lamports, &mut data, &owner);
        assert!(verify_signer(&account).is_err());
    }

    #[test]
    fn test_verify_writable_success() {
        let key = Pubkey::new_unique();
        let mut lamports = 0;
        let mut data = [];
        let owner = system_program::id();
        
        let account = create_test_account_info(&key, false, true, &mut lamports, &mut data, &owner);
        assert!(verify_writable(&account).is_ok());
    }

    #[test]
    fn test_verify_writable_failure() {
        let key = Pubkey::new_unique();
        let mut lamports = 0;
        let mut data = [];
        let owner = system_program::id();
        
        let account = create_test_account_info(&key, false, false, &mut lamports, &mut data, &owner);
        assert!(verify_writable(&account).is_err());
    }

    #[test]
    fn test_verify_account_owner_success() {
        let key = Pubkey::new_unique();
        let mut lamports = 0;
        let mut data = [];
        let owner = system_program::id();
        
        let account = create_test_account_info(&key, false, false, &mut lamports, &mut data, &owner);
        assert!(verify_account_owner(&account, &system_program::id()).is_ok());
    }

    #[test]
    fn test_verify_account_owner_failure() {
        let key = Pubkey::new_unique();
        let mut lamports = 0;
        let mut data = [];
        let owner = system_program::id();
        let wrong_owner = Pubkey::new_unique();
        
        let account = create_test_account_info(&key, false, false, &mut lamports, &mut data, &owner);
        assert!(verify_account_owner(&account, &wrong_owner).is_err());
    }
}
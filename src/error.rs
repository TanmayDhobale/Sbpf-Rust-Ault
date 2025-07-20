use solana_program::{
    decode_error::DecodeError,
    msg,
    program_error::{PrintProgramError, ProgramError},
};
use thiserror::Error;

/// Custom error types for the vault program
#[derive(Error, Debug, Copy, Clone)]
pub enum VaultError {
    /// Insufficient funds for withdrawal
    #[error("Insufficient funds")]
    InsufficientFunds,
    
    /// Unauthorized access attempt
    #[error("Unauthorized access")]
    UnauthorizedAccess,
    
    /// Invalid input parameters
    #[error("Invalid input")]
    InvalidInput,
    
    /// Vault is closed
    #[error("Vault is closed")]
    VaultClosed,
    
    /// Invalid token account
    #[error("Invalid token account")]
    InvalidTokenAccount,
    
    /// Invalid mint
    #[error("Invalid mint")]
    InvalidMint,
    
    /// Arithmetic overflow
    #[error("Arithmetic overflow")]
    ArithmeticOverflow,
    
    /// Account not initialized
    #[error("Account not initialized")]
    AccountNotInitialized,
}

impl From<VaultError> for ProgramError {
    fn from(e: VaultError) -> Self {
        ProgramError::Custom(e as u32)
    }
}

impl<T> DecodeError<T> for VaultError {
    fn type_of() -> &'static str {
        "VaultError"
    }
}

impl PrintProgramError for VaultError {
    fn print<E>(&self) {
        match self {
            VaultError::InsufficientFunds => msg!("Error: Insufficient funds for withdrawal"),
            VaultError::UnauthorizedAccess => msg!("Error: Unauthorized access attempt"),
            VaultError::InvalidInput => msg!("Error: Invalid input parameters provided"),
            VaultError::VaultClosed => msg!("Error: Vault is closed, no operations allowed"),
            VaultError::InvalidTokenAccount => msg!("Error: Invalid token account provided"),
            VaultError::InvalidMint => msg!("Error: Invalid mint provided"),
            VaultError::ArithmeticOverflow => msg!("Error: Arithmetic overflow occurred"),
            VaultError::AccountNotInitialized => msg!("Error: Account not properly initialized"),
        }
    }
}
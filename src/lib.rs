use solana_program::{
    account_info::AccountInfo,
    entrypoint,
    entrypoint::ProgramResult,
    pubkey::Pubkey,
};

// Program modules
pub mod error;
pub mod instruction;
pub mod processor;
pub mod state;
pub mod utils;

// Re-exports for external use (will be uncommented as modules are implemented)
pub use error::VaultError;
pub use instruction::VaultInstruction;
pub use state::{UserBalance, VaultState};

// Program entrypoint
#[cfg(not(feature = "no-entrypoint"))]
entrypoint!(process_instruction);

/// Main program entry point
pub fn process_instruction(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    instruction_data: &[u8],
) -> ProgramResult {
    processor::process_instruction(program_id, accounts, instruction_data)
}

// Declare program ID (this would be set after deployment)
solana_program::declare_id!("VauLTsyDxEHVqb8rTzQNiubxvNjwfqzMsLkU8aTzrNc");
use borsh::{BorshDeserialize, BorshSerialize};
use solana_program::{
    instruction::{AccountMeta, Instruction},
    program_error::ProgramError,
    pubkey::Pubkey,
    system_program,
    sysvar,
};

use crate::error::VaultError;

/// Instructions supported by the vault program
#[derive(BorshSerialize, BorshDeserialize, Debug, Clone, PartialEq)]
pub enum VaultInstruction {
    /// Initialize a new vault
    /// 
    /// Accounts expected:
    /// 0. [signer, writable] Vault owner
    /// 1. [writable] Vault state account (PDA)
    /// 2. [writable] Vault token account
    /// 3. [] Token mint
    /// 4. [] SPL Token program
    /// 5. [] System program
    /// 6. [] Rent sysvar
    Initialize,

    /// Deposit SPL tokens into the vault
    /// 
    /// Accounts expected:
    /// 0. [signer, writable] User account
    /// 1. [writable] User token account
    /// 2. [writable] Vault token account
    /// 3. [writable] Vault state account
    /// 4. [writable] User balance account (PDA)
    /// 5. [] SPL Token program
    /// 6. [] System program (for PDA creation if needed)
    Deposit { amount: u64 },

    /// Withdraw SPL tokens from the vault
    /// 
    /// Accounts expected:
    /// 0. [signer, writable] User account
    /// 1. [writable] User token account
    /// 2. [writable] Vault token account
    /// 3. [writable] Vault state account
    /// 4. [writable] User balance account (PDA)
    /// 5. [] SPL Token program
    Withdraw { amount: u64 },

    /// Owner withdraws all funds from the vault
    /// 
    /// Accounts expected:
    /// 0. [signer, writable] Vault owner
    /// 1. [writable] Owner token account
    /// 2. [writable] Vault token account
    /// 3. [writable] Vault state account
    /// 4. [] SPL Token program
    WithdrawAll,

    /// Close the vault (owner only)
    /// 
    /// Accounts expected:
    /// 0. [signer, writable] Vault owner
    /// 1. [writable] Owner token account (to receive remaining tokens)
    /// 2. [writable] Vault token account
    /// 3. [writable] Vault state account
    /// 4. [] SPL Token program
    Close,
}

impl VaultInstruction {
    /// Create an Initialize instruction
    pub fn initialize(
        program_id: &Pubkey,
        owner: &Pubkey,
        vault_state: &Pubkey,
        vault_token_account: &Pubkey,
        token_mint: &Pubkey,
    ) -> Instruction {
        let accounts = vec![
            AccountMeta::new(*owner, true),
            AccountMeta::new(*vault_state, false),
            AccountMeta::new(*vault_token_account, false),
            AccountMeta::new_readonly(*token_mint, false),
            AccountMeta::new_readonly(spl_token::id(), false),
            AccountMeta::new_readonly(system_program::id(), false),
            AccountMeta::new_readonly(sysvar::rent::id(), false),
        ];

        Instruction {
            program_id: *program_id,
            accounts,
            data: VaultInstruction::Initialize.try_to_vec().unwrap(),
        }
    }

    /// Create a Deposit instruction
    pub fn deposit(
        program_id: &Pubkey,
        user: &Pubkey,
        user_token_account: &Pubkey,
        vault_token_account: &Pubkey,
        vault_state: &Pubkey,
        user_balance_account: &Pubkey,
        amount: u64,
    ) -> Instruction {
        let accounts = vec![
            AccountMeta::new(*user, true),
            AccountMeta::new(*user_token_account, false),
            AccountMeta::new(*vault_token_account, false),
            AccountMeta::new(*vault_state, false),
            AccountMeta::new(*user_balance_account, false),
            AccountMeta::new_readonly(spl_token::id(), false),
            AccountMeta::new_readonly(system_program::id(), false),
        ];

        Instruction {
            program_id: *program_id,
            accounts,
            data: VaultInstruction::Deposit { amount }.try_to_vec().unwrap(),
        }
    }

    /// Create a Withdraw instruction
    pub fn withdraw(
        program_id: &Pubkey,
        user: &Pubkey,
        user_token_account: &Pubkey,
        vault_token_account: &Pubkey,
        vault_state: &Pubkey,
        user_balance_account: &Pubkey,
        amount: u64,
    ) -> Instruction {
        let accounts = vec![
            AccountMeta::new(*user, true),
            AccountMeta::new(*user_token_account, false),
            AccountMeta::new(*vault_token_account, false),
            AccountMeta::new(*vault_state, false),
            AccountMeta::new(*user_balance_account, false),
            AccountMeta::new_readonly(spl_token::id(), false),
        ];

        Instruction {
            program_id: *program_id,
            accounts,
            data: VaultInstruction::Withdraw { amount }.try_to_vec().unwrap(),
        }
    }

    /// Create a WithdrawAll instruction
    pub fn withdraw_all(
        program_id: &Pubkey,
        owner: &Pubkey,
        owner_token_account: &Pubkey,
        vault_token_account: &Pubkey,
        vault_state: &Pubkey,
    ) -> Instruction {
        let accounts = vec![
            AccountMeta::new(*owner, true),
            AccountMeta::new(*owner_token_account, false),
            AccountMeta::new(*vault_token_account, false),
            AccountMeta::new(*vault_state, false),
            AccountMeta::new_readonly(spl_token::id(), false),
        ];

        Instruction {
            program_id: *program_id,
            accounts,
            data: VaultInstruction::WithdrawAll.try_to_vec().unwrap(),
        }
    }

    /// Create a Close instruction
    pub fn close(
        program_id: &Pubkey,
        owner: &Pubkey,
        owner_token_account: &Pubkey,
        vault_token_account: &Pubkey,
        vault_state: &Pubkey,
    ) -> Instruction {
        let accounts = vec![
            AccountMeta::new(*owner, true),
            AccountMeta::new(*owner_token_account, false),
            AccountMeta::new(*vault_token_account, false),
            AccountMeta::new(*vault_state, false),
            AccountMeta::new_readonly(spl_token::id(), false),
        ];

        Instruction {
            program_id: *program_id,
            accounts,
            data: VaultInstruction::Close.try_to_vec().unwrap(),
        }
    }
}

/// Parse instruction data into VaultInstruction
pub fn unpack(input: &[u8]) -> Result<VaultInstruction, ProgramError> {
    if input.is_empty() {
        return Err(VaultError::InvalidInput.into());
    }

    VaultInstruction::try_from_slice(input).map_err(|_| VaultError::InvalidInput.into())
}

/// Validate instruction data format and size
pub fn validate_instruction_data(data: &[u8]) -> Result<(), ProgramError> {
    if data.is_empty() {
        return Err(VaultError::InvalidInput.into());
    }

    // Try to deserialize to validate format
    match VaultInstruction::try_from_slice(data) {
        Ok(instruction) => {
            // Additional validation based on instruction type
            match instruction {
                VaultInstruction::Deposit { amount } | VaultInstruction::Withdraw { amount } => {
                    if amount == 0 {
                        return Err(VaultError::InvalidInput.into());
                    }
                }
                _ => {}
            }
            Ok(())
        }
        Err(_) => Err(VaultError::InvalidInput.into()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_instruction_serialization() {
        let deposit = VaultInstruction::Deposit { amount: 1000 };
        let serialized = deposit.try_to_vec().unwrap();
        let deserialized = VaultInstruction::try_from_slice(&serialized).unwrap();
        assert_eq!(deposit, deserialized);
    }

    #[test]
    fn test_unpack_valid_instruction() {
        let instruction = VaultInstruction::Initialize;
        let data = instruction.try_to_vec().unwrap();
        let unpacked = unpack(&data).unwrap();
        assert_eq!(instruction, unpacked);
    }

    #[test]
    fn test_unpack_empty_data() {
        let result = unpack(&[]);
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_zero_amount() {
        let instruction = VaultInstruction::Deposit { amount: 0 };
        let data = instruction.try_to_vec().unwrap();
        let result = validate_instruction_data(&data);
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_valid_amount() {
        let instruction = VaultInstruction::Deposit { amount: 1000 };
        let data = instruction.try_to_vec().unwrap();
        let result = validate_instruction_data(&data);
        assert!(result.is_ok());
    }
}
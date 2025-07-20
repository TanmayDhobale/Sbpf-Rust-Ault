use borsh::{BorshDeserialize, BorshSerialize};
use solana_program::{
    account_info::{next_account_info, AccountInfo},
    entrypoint::ProgramResult,
    msg,
    program::invoke_signed,
    program_error::ProgramError,
    program_pack::Pack,
    pubkey::Pubkey,
    rent::Rent,
    system_instruction,
    sysvar::Sysvar,
};

use crate::{
    error::VaultError,
    instruction::{unpack, VaultInstruction},
    state::{VaultState, UserBalance},
    utils::{derive_vault_state_pda, derive_user_balance_pda, verify_signer, verify_token_mint},
};

/// Helper function for logging buffer state for debugging
fn log_buffer_state(data: &[u8], operation: &str) {
    msg!("{}: Buffer length: {}", operation, data.len());
    msg!("{}: Expected VaultState size: {}", operation, VaultState::SIZE);
    if !data.is_empty() {
        let preview_len = 20.min(data.len());
        msg!("{}: First {} bytes: {:?}", operation, preview_len, &data[..preview_len]);
        if data.len() > 20 {
            let tail_start = data.len().saturating_sub(20);
            msg!("{}: Last 20 bytes: {:?}", operation, &data[tail_start..]);
        }
    }
}

/// Validates account data buffer for vault state operations
fn validate_vault_buffer(
    account_data: &[u8],
    expected_size: usize,
    operation: &str,
) -> Result<(), ProgramError> {
    if account_data.len() != expected_size {
        msg!("{}: Buffer size mismatch - expected: {}, actual: {}", 
             operation, expected_size, account_data.len());
        log_buffer_state(account_data, operation);
        return Err(VaultError::InvalidInput.into());
    }
    Ok(())
}

/// Enhanced helper for vault state serialization with comprehensive validation
fn serialize_vault_state_safe(
    vault_state: &VaultState,
    vault_state_data: &mut [u8],
    operation: &str,
) -> Result<(), ProgramError> {
    msg!("{}: Starting serialization", operation);
    msg!("{}: Buffer length: {}, Expected size: {}", 
         operation, vault_state_data.len(), VaultState::SIZE);
    
    // Validate vault state before serialization
    vault_state.validate().map_err(|err| {
        msg!("{}: Vault state validation failed: {}", operation, err);
        VaultError::InvalidInput
    })?;
    
    // Serialize the vault state
    let serialized_data = vault_state.try_to_vec()
        .map_err(|e| {
            msg!("{}: Failed to serialize vault state: {}", operation, e);
            VaultError::InvalidInput
        })?;
    
    msg!("{}: Serialized data length: {}", operation, serialized_data.len());
    
    // Validate serialized data size
    if serialized_data.len() != VaultState::SIZE {
        msg!("{}: Serialization size mismatch - expected: {}, got: {}", 
             operation, VaultState::SIZE, serialized_data.len());
        return Err(VaultError::InvalidInput.into());
    }
    
    // Validate buffer size
    if vault_state_data.len() < serialized_data.len() {
        msg!("{}: Account data buffer too small - required: {}, available: {}", 
             operation, serialized_data.len(), vault_state_data.len());
        return Err(VaultError::InvalidInput.into());
    }
    
    // Copy the serialized data to the exact required space
    vault_state_data[..serialized_data.len()].copy_from_slice(&serialized_data);
    
    msg!("{}: Successfully serialized vault state", operation);
    Ok(())
}

/// Helper for safe vault state deserialization with error recovery
fn deserialize_vault_state_safe(
    vault_state_data: &[u8],
    operation: &str,
) -> Result<VaultState, ProgramError> {
    msg!("{}: Starting deserialization", operation);
    
    // Log buffer state for debugging
    log_buffer_state(vault_state_data, operation);
    
    // Validate buffer size before deserialization
    validate_vault_buffer(vault_state_data, VaultState::SIZE, operation)?;
    
    // Attempt deserialization
    let vault_state = VaultState::try_from_slice(vault_state_data)
        .map_err(|e| {
            msg!("{}: Failed to deserialize vault state: {}", operation, e);
            msg!("{}: This may indicate data corruption or format mismatch", operation);
            log_buffer_state(vault_state_data, operation);
            VaultError::AccountNotInitialized
        })?;
    
    // Validate deserialized state
    vault_state.validate().map_err(|err| {
        msg!("{}: Deserialized vault state validation failed: {}", operation, err);
        VaultError::InvalidInput
    })?;
    
    msg!("{}: Successfully deserialized vault state", operation);
    Ok(vault_state)
}

/// Helper for safe user balance deserialization with error recovery
fn deserialize_user_balance_safe(
    user_balance_data: &[u8],
    operation: &str,
) -> Result<UserBalance, ProgramError> {
    msg!("{}: Starting user balance deserialization", operation);
    
    // Log buffer state for debugging
    msg!("{}: User balance buffer length: {}", operation, user_balance_data.len());
    msg!("{}: Expected UserBalance size: {}", operation, UserBalance::SIZE);
    
    if !user_balance_data.is_empty() {
        let preview_len = 20.min(user_balance_data.len());
        msg!("{}: First {} bytes: {:?}", operation, preview_len, &user_balance_data[..preview_len]);
    }
    
    // Validate buffer size before deserialization
    if user_balance_data.len() != UserBalance::SIZE {
        msg!("{}: User balance buffer size mismatch - expected: {}, actual: {}", 
             operation, UserBalance::SIZE, user_balance_data.len());
        return Err(VaultError::AccountNotInitialized.into());
    }
    
    // Attempt deserialization
    let user_balance = UserBalance::try_from_slice(user_balance_data)
        .map_err(|e| {
            msg!("{}: Failed to deserialize user balance: {}", operation, e);
            msg!("{}: This may indicate data corruption or format mismatch", operation);
            VaultError::AccountNotInitialized
        })?;
    
    // Validate deserialized state
    user_balance.validate().map_err(|err| {
        msg!("{}: Deserialized user balance validation failed: {}", operation, err);
        VaultError::InvalidInput
    })?;
    
    msg!("{}: Successfully deserialized user balance", operation);
    Ok(user_balance)
}

/// Enhanced helper for user balance serialization with comprehensive validation
fn serialize_user_balance_safe(
    user_balance: &UserBalance,
    user_balance_data: &mut [u8],
    operation: &str,
) -> Result<(), ProgramError> {
    msg!("{}: Starting user balance serialization", operation);
    msg!("{}: Buffer length: {}, Expected size: {}", 
         operation, user_balance_data.len(), UserBalance::SIZE);
    
    // Validate user balance before serialization
    user_balance.validate().map_err(|err| {
        msg!("{}: User balance validation failed: {}", operation, err);
        VaultError::InvalidInput
    })?;
    
    // Serialize the user balance
    let serialized_data = user_balance.try_to_vec()
        .map_err(|e| {
            msg!("{}: Failed to serialize user balance: {}", operation, e);
            VaultError::InvalidInput
        })?;
    
    msg!("{}: Serialized user balance data length: {}", operation, serialized_data.len());
    
    // Validate serialized data size
    if serialized_data.len() != UserBalance::SIZE {
        msg!("{}: User balance serialization size mismatch - expected: {}, got: {}", 
             operation, UserBalance::SIZE, serialized_data.len());
        return Err(VaultError::InvalidInput.into());
    }
    
    // Validate buffer size
    if user_balance_data.len() < serialized_data.len() {
        msg!("{}: User balance account data buffer too small - required: {}, available: {}", 
             operation, serialized_data.len(), user_balance_data.len());
        return Err(VaultError::InvalidInput.into());
    }
    
    // Copy the serialized data to the exact required space
    user_balance_data[..serialized_data.len()].copy_from_slice(&serialized_data);
    
    msg!("{}: Successfully serialized user balance", operation);
    Ok(())
}

/// Legacy helper function for backward compatibility - delegates to safe version
fn serialize_vault_state(
    vault_state: &VaultState,
    vault_state_data: &mut [u8],
    operation: &str,
) -> Result<(), ProgramError> {
    serialize_vault_state_safe(vault_state, vault_state_data, operation)
}

pub fn process_instruction(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    instruction_data: &[u8],
) -> ProgramResult {
    let instruction = unpack(instruction_data)?;
    
    match instruction {
        VaultInstruction::Initialize => process_initialize(program_id, accounts),
        VaultInstruction::Deposit { amount } => {
            process_deposit(program_id, accounts, amount)
        }
        VaultInstruction::Withdraw { amount } => {
            process_withdraw(program_id, accounts, amount)
        }
        VaultInstruction::WithdrawAll => {
            process_withdraw_all(program_id, accounts)
        }
        VaultInstruction::Close => {
            process_close(program_id, accounts)
        }
    }
}

/// Process Initialize instruction
/// Creates a new vault with the specified owner and token mint
pub fn process_initialize(program_id: &Pubkey, accounts: &[AccountInfo]) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();
    
    // Validate minimum number of accounts
    if accounts.len() < 7 {
        msg!("Initialize: Insufficient accounts provided");
        return Err(VaultError::InvalidInput.into());
    }
    
    // Expected accounts:
    // 0. [signer, writable] Vault owner
    // 1. [writable] Vault state account (PDA)
    // 2. [writable] Vault token account
    // 3. [] Token mint
    // 4. [] SPL Token program
    // 5. [] System program
    // 6. [] Rent sysvar
    let owner_info = next_account_info(account_info_iter)?;
    let vault_state_info = next_account_info(account_info_iter)?;
    let vault_token_account_info = next_account_info(account_info_iter)?;
    let token_mint_info = next_account_info(account_info_iter)?;
    let token_program_info = next_account_info(account_info_iter)?;
    let system_program_info = next_account_info(account_info_iter)?;
    let rent_info = next_account_info(account_info_iter)?;

    // Comprehensive account validation
    
    // Verify owner is signer and writable
    if !owner_info.is_signer {
        msg!("Initialize: Owner must be signer");
        return Err(VaultError::UnauthorizedAccess.into());
    }
    if !owner_info.is_writable {
        msg!("Initialize: Owner account must be writable");
        return Err(VaultError::InvalidInput.into());
    }
    
    // Verify vault state account is writable
    if !vault_state_info.is_writable {
        msg!("Initialize: Vault state account must be writable");
        return Err(VaultError::InvalidInput.into());
    }
    
    // Verify vault token account is writable and owned by token program
    if !vault_token_account_info.is_writable {
        msg!("Initialize: Vault token account must be writable");
        return Err(VaultError::InvalidInput.into());
    }
    if vault_token_account_info.owner != &spl_token::id() {
        msg!("Initialize: Vault token account must be owned by SPL Token program");
        return Err(VaultError::InvalidTokenAccount.into());
    }
    
    // Verify token mint is valid and owned by token program
    if token_mint_info.owner != &spl_token::id() {
        msg!("Initialize: Token mint must be owned by SPL Token program");
        return Err(VaultError::InvalidMint.into());
    }
    
    // Verify token mint structure
    let mint_data = token_mint_info.try_borrow_data()?;
    if mint_data.len() != spl_token::state::Mint::LEN {
        msg!("Initialize: Invalid token mint data length");
        return Err(VaultError::InvalidMint.into());
    }
    let mint = spl_token::state::Mint::unpack(&mint_data)
        .map_err(|_| {
            msg!("Initialize: Failed to unpack token mint");
            VaultError::InvalidMint
        })?;
    drop(mint_data);
    
    // Verify vault token account matches the mint
    let vault_token_data = vault_token_account_info.try_borrow_data()?;
    if vault_token_data.len() != spl_token::state::Account::LEN {
        msg!("Initialize: Invalid vault token account data length");
        return Err(VaultError::InvalidTokenAccount.into());
    }
    let vault_token_account = spl_token::state::Account::unpack(&vault_token_data)
        .map_err(|_| {
            msg!("Initialize: Failed to unpack vault token account");
            VaultError::InvalidTokenAccount
        })?;
    
    if vault_token_account.mint != *token_mint_info.key {
        msg!("Initialize: Vault token account mint mismatch");
        return Err(VaultError::InvalidMint.into());
    }
    drop(vault_token_data);
    
    // Verify program accounts
    if token_program_info.key != &spl_token::id() {
        msg!("Initialize: Invalid SPL Token program");
        return Err(VaultError::InvalidTokenAccount.into());
    }
    
    if system_program_info.key != &solana_program::system_program::id() {
        msg!("Initialize: Invalid System program");
        return Err(VaultError::InvalidInput.into());
    }
    
    if rent_info.key != &solana_program::sysvar::rent::id() {
        msg!("Initialize: Invalid Rent sysvar");
        return Err(VaultError::InvalidInput.into());
    }
    
    // Derive and verify vault state PDA
    let (vault_state_pda, vault_state_bump) = derive_vault_state_pda(
        program_id,
        owner_info.key,
        token_mint_info.key,
    )?;
    
    if vault_state_pda != *vault_state_info.key {
        msg!("Initialize: Vault state PDA mismatch. Expected: {}, Got: {}", 
             vault_state_pda, vault_state_info.key);
        return Err(VaultError::InvalidInput.into());
    }
    
    // Verify vault state account is uninitialized
    if vault_state_info.owner != &solana_program::system_program::id() {
        msg!("Initialize: Vault state account already initialized");
        return Err(VaultError::AccountNotInitialized.into());
    }
    
    if vault_state_info.data_len() != 0 {
        msg!("Initialize: Vault state account must be empty");
        return Err(VaultError::AccountNotInitialized.into());
    }
    
    // Get rent and validate rent exemption
    let rent = Rent::from_account_info(rent_info)?;
    
    // Calculate space needed for vault state
    let vault_state_space = VaultState::SIZE;
    let vault_state_lamports = rent.minimum_balance(vault_state_space);
    
    // Verify owner has sufficient lamports
    if owner_info.lamports() < vault_state_lamports {
        msg!("Initialize: Insufficient lamports for rent exemption. Required: {}, Available: {}", 
             vault_state_lamports, owner_info.lamports());
        return Err(VaultError::InvalidInput.into());
    }
    
    // Create vault state account
    let create_vault_state_ix = system_instruction::create_account(
        owner_info.key,
        vault_state_info.key,
        vault_state_lamports,
        vault_state_space as u64,
        program_id,
    );
    
    let vault_state_seeds = &[
        crate::utils::VAULT_SEED,
        owner_info.key.as_ref(),
        token_mint_info.key.as_ref(),
        &[vault_state_bump],
    ];
    
    invoke_signed(
        &create_vault_state_ix,
        &[
            owner_info.clone(),
            vault_state_info.clone(),
            system_program_info.clone(),
        ],
        &[vault_state_seeds],
    ).map_err(|e| {
        msg!("Initialize: Failed to create vault state account: {}", e);
        e
    })?;
    
    // Initialize vault state data
    let vault_state = VaultState::new(
        *owner_info.key,
        *token_mint_info.key,
        *vault_token_account_info.key,
        vault_state_bump,
    );
    
    // Validate the vault state
    vault_state.validate().map_err(|err| {
        msg!("Initialize: Vault state validation failed: {}", err);
        VaultError::InvalidInput
    })?;
    
    // Serialize and store vault state
    let mut vault_state_data = vault_state_info.try_borrow_mut_data()
        .map_err(|e| {
            msg!("Initialize: Failed to borrow vault state data: {}", e);
            VaultError::InvalidInput
        })?;
    
    msg!("Initialize: Account data length before serialization: {}", vault_state_data.len());
    
    serialize_vault_state(&vault_state, &mut *vault_state_data, "Initialize")?;
    
    msg!(
        "Vault initialized successfully. Owner: {}, Mint: {}, Token Account: {}, Bump: {}",
        owner_info.key,
        token_mint_info.key,
        vault_token_account_info.key,
        vault_state_bump
    );
    
    Ok(())
}

/// Process Deposit instruction
/// Allows users to deposit SPL tokens into the vault
pub fn process_deposit(program_id: &Pubkey, accounts: &[AccountInfo], amount: u64) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();
    
    // Validate minimum number of accounts
    if accounts.len() < 7 {
        msg!("Deposit: Insufficient accounts provided");
        return Err(VaultError::InvalidInput.into());
    }
    
    // Expected accounts:
    // 0. [signer, writable] User account
    // 1. [writable] User token account
    // 2. [writable] Vault token account
    // 3. [writable] Vault state account
    // 4. [writable] User balance account (PDA)
    // 5. [] SPL Token program
    // 6. [] System program (for PDA creation if needed)
    let user_info = next_account_info(account_info_iter)?;
    let user_token_account_info = next_account_info(account_info_iter)?;
    let vault_token_account_info = next_account_info(account_info_iter)?;
    let vault_state_info = next_account_info(account_info_iter)?;
    let user_balance_info = next_account_info(account_info_iter)?;
    let token_program_info = next_account_info(account_info_iter)?;
    let system_program_info = next_account_info(account_info_iter)?;

    // Validate deposit amount
    if amount == 0 {
        msg!("Deposit: Amount must be greater than zero");
        return Err(VaultError::InvalidInput.into());
    }

    // Verify user is signer
    if !user_info.is_signer {
        msg!("Deposit: User must be signer");
        return Err(VaultError::UnauthorizedAccess.into());
    }

    // Verify accounts are writable
    if !user_token_account_info.is_writable {
        msg!("Deposit: User token account must be writable");
        return Err(VaultError::InvalidInput.into());
    }
    if !vault_token_account_info.is_writable {
        msg!("Deposit: Vault token account must be writable");
        return Err(VaultError::InvalidInput.into());
    }
    if !vault_state_info.is_writable {
        msg!("Deposit: Vault state account must be writable");
        return Err(VaultError::InvalidInput.into());
    }
    if !user_balance_info.is_writable {
        msg!("Deposit: User balance account must be writable");
        return Err(VaultError::InvalidInput.into());
    }

    // Verify program accounts
    if token_program_info.key != &spl_token::id() {
        msg!("Deposit: Invalid SPL Token program");
        return Err(VaultError::InvalidTokenAccount.into());
    }
    if system_program_info.key != &solana_program::system_program::id() {
        msg!("Deposit: Invalid System program");
        return Err(VaultError::InvalidInput.into());
    }

    // Load and validate vault state
    let vault_state_data = vault_state_info.try_borrow_data()?;
    let mut vault_state = deserialize_vault_state_safe(&vault_state_data, "Deposit")?;
    drop(vault_state_data); // Drop the read borrow early

    // Check if vault is operational
    if !vault_state.is_operational() {
        msg!("Deposit: Vault is closed");
        return Err(VaultError::VaultClosed.into());
    }

    // Verify vault state account ownership
    if vault_state_info.owner != program_id {
        msg!("Deposit: Vault state account not owned by program");
        return Err(VaultError::InvalidInput.into());
    }

    // Verify token accounts are owned by SPL Token program
    if user_token_account_info.owner != &spl_token::id() {
        msg!("Deposit: User token account must be owned by SPL Token program");
        return Err(VaultError::InvalidTokenAccount.into());
    }
    if vault_token_account_info.owner != &spl_token::id() {
        msg!("Deposit: Vault token account must be owned by SPL Token program");
        return Err(VaultError::InvalidTokenAccount.into());
    }

    // Verify token accounts match the vault's mint
    let user_token_data = user_token_account_info.try_borrow_data()?;
    let user_token_account = spl_token::state::Account::unpack(&user_token_data)
        .map_err(|_| {
            msg!("Deposit: Failed to unpack user token account");
            VaultError::InvalidTokenAccount
        })?;
    
    if user_token_account.mint != vault_state.token_mint {
        msg!("Deposit: User token account mint mismatch");
        return Err(VaultError::InvalidMint.into());
    }

    // Verify user has sufficient balance
    if user_token_account.amount < amount {
        msg!("Deposit: Insufficient user token balance. Required: {}, Available: {}", 
             amount, user_token_account.amount);
        return Err(VaultError::InsufficientFunds.into());
    }
    drop(user_token_data);

    // Verify vault token account
    let vault_token_data = vault_token_account_info.try_borrow_data()?;
    let vault_token_account = spl_token::state::Account::unpack(&vault_token_data)
        .map_err(|_| {
            msg!("Deposit: Failed to unpack vault token account");
            VaultError::InvalidTokenAccount
        })?;
    
    if vault_token_account.mint != vault_state.token_mint {
        msg!("Deposit: Vault token account mint mismatch");
        return Err(VaultError::InvalidMint.into());
    }
    drop(vault_token_data);

    // Derive and verify user balance PDA
    let (user_balance_pda, user_balance_bump) = derive_user_balance_pda(
        program_id,
        user_info.key,
        vault_state_info.key,
    )?;

    if user_balance_pda != *user_balance_info.key {
        msg!("Deposit: User balance PDA mismatch. Expected: {}, Got: {}", 
             user_balance_pda, user_balance_info.key);
        return Err(VaultError::InvalidInput.into());
    }

    // Handle user balance account creation or loading
    let mut user_balance = if user_balance_info.owner == &solana_program::system_program::id() {
        // Account doesn't exist, create it
        let rent = Rent::get()?;
        let user_balance_space = UserBalance::SIZE;
        let user_balance_lamports = rent.minimum_balance(user_balance_space);

        let create_user_balance_ix = system_instruction::create_account(
            user_info.key,
            user_balance_info.key,
            user_balance_lamports,
            user_balance_space as u64,
            program_id,
        );

        let user_balance_seeds = &[
            crate::utils::USER_BALANCE_SEED,
            user_info.key.as_ref(),
            vault_state_info.key.as_ref(),
            &[user_balance_bump],
        ];

        invoke_signed(
            &create_user_balance_ix,
            &[
                user_info.clone(),
                user_balance_info.clone(),
                system_program_info.clone(),
            ],
            &[user_balance_seeds],
        ).map_err(|e| {
            msg!("Deposit: Failed to create user balance account: {}", e);
            e
        })?;

        // Initialize new user balance
        UserBalance::new(*user_info.key, *vault_state_info.key, user_balance_bump)
    } else if user_balance_info.owner == program_id {
        // Account exists, load it
        let user_balance_data = user_balance_info.try_borrow_data()?;
        deserialize_user_balance_safe(&user_balance_data, "Deposit")?
    } else {
        msg!("Deposit: User balance account has invalid owner");
        return Err(VaultError::InvalidInput.into());
    };

    // Validate user balance account
    user_balance.validate().map_err(|err| {
        msg!("Deposit: User balance validation failed: {}", err);
        VaultError::InvalidInput
    })?;

    // Transfer tokens from user to vault
    let transfer_ix = spl_token::instruction::transfer(
        &spl_token::id(),
        user_token_account_info.key,
        vault_token_account_info.key,
        user_info.key,
        &[],
        amount,
    )?;

    solana_program::program::invoke(
        &transfer_ix,
        &[
            user_token_account_info.clone(),
            vault_token_account_info.clone(),
            user_info.clone(),
            token_program_info.clone(),
        ],
    ).map_err(|e| {
        msg!("Deposit: Token transfer failed: {}", e);
        e
    })?;

    // Update user balance with overflow protection
    user_balance.add_balance(amount).map_err(|err| {
        msg!("Deposit: Failed to update user balance: {}", err);
        VaultError::ArithmeticOverflow
    })?;

    // Update vault total deposited with overflow protection
    vault_state.add_deposit(amount).map_err(|err| {
        msg!("Deposit: Failed to update vault total: {}", err);
        VaultError::ArithmeticOverflow
    })?;

    // Save updated user balance
    let mut user_balance_data = user_balance_info.try_borrow_mut_data()?;
    serialize_user_balance_safe(&user_balance, &mut *user_balance_data, "Deposit")?;
    drop(user_balance_data);

    // Save updated vault state
    let mut vault_state_data = vault_state_info.try_borrow_mut_data()?; // Borrow for writing
    
    serialize_vault_state(&vault_state, &mut *vault_state_data, "Deposit")?;

    msg!(
        "Deposit successful. User: {}, Amount: {}, New Balance: {}, Vault Total: {}",
        user_info.key,
        amount,
        user_balance.balance,
        vault_state.total_deposited
    );

    Ok(())
}

/// Process Withdraw instruction
/// Allows users to withdraw SPL tokens from the vault
pub fn process_withdraw(program_id: &Pubkey, accounts: &[AccountInfo], amount: u64) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();
    
    // Validate minimum number of accounts
    if accounts.len() < 6 {
        msg!("Withdraw: Insufficient accounts provided");
        return Err(VaultError::InvalidInput.into());
    }
    
    // Expected accounts:
    // 0. [signer, writable] User account
    // 1. [writable] User token account
    // 2. [writable] Vault token account
    // 3. [writable] Vault state account
    // 4. [writable] User balance account (PDA)
    // 5. [] SPL Token program
    let user_info = next_account_info(account_info_iter)?;
    let user_token_account_info = next_account_info(account_info_iter)?;
    let vault_token_account_info = next_account_info(account_info_iter)?;
    let vault_state_info = next_account_info(account_info_iter)?;
    let user_balance_info = next_account_info(account_info_iter)?;
    let token_program_info = next_account_info(account_info_iter)?;

    // Validate withdrawal amount
    if amount == 0 {
        msg!("Withdraw: Amount must be greater than zero");
        return Err(VaultError::InvalidInput.into());
    }

    // Verify user is signer
    if !user_info.is_signer {
        msg!("Withdraw: User must be signer");
        return Err(VaultError::UnauthorizedAccess.into());
    }

    // Verify accounts are writable
    if !user_token_account_info.is_writable {
        msg!("Withdraw: User token account must be writable");
        return Err(VaultError::InvalidInput.into());
    }
    if !vault_token_account_info.is_writable {
        msg!("Withdraw: Vault token account must be writable");
        return Err(VaultError::InvalidInput.into());
    }
    if !vault_state_info.is_writable {
        msg!("Withdraw: Vault state account must be writable");
        return Err(VaultError::InvalidInput.into());
    }
    if !user_balance_info.is_writable {
        msg!("Withdraw: User balance account must be writable");
        return Err(VaultError::InvalidInput.into());
    }

    // Verify program accounts
    if token_program_info.key != &spl_token::id() {
        msg!("Withdraw: Invalid SPL Token program");
        return Err(VaultError::InvalidTokenAccount.into());
    }

    // Load and validate vault state
    let vault_state_data = vault_state_info.try_borrow_data()?;
    let mut vault_state = deserialize_vault_state_safe(&vault_state_data, "Withdraw")?;
    drop(vault_state_data); // Drop the read borrow early

    // Check if vault is operational
    if !vault_state.is_operational() {
        msg!("Withdraw: Vault is closed");
        return Err(VaultError::VaultClosed.into());
    }

    // Verify vault state account ownership
    if vault_state_info.owner != program_id {
        msg!("Withdraw: Vault state account not owned by program");
        return Err(VaultError::InvalidInput.into());
    }

    // Verify token accounts are owned by SPL Token program
    if user_token_account_info.owner != &spl_token::id() {
        msg!("Withdraw: User token account must be owned by SPL Token program");
        return Err(VaultError::InvalidTokenAccount.into());
    }
    if vault_token_account_info.owner != &spl_token::id() {
        msg!("Withdraw: Vault token account must be owned by SPL Token program");
        return Err(VaultError::InvalidTokenAccount.into());
    }

    // Verify token accounts match the vault's mint
    let user_token_data = user_token_account_info.try_borrow_data()?;
    let user_token_account = spl_token::state::Account::unpack(&user_token_data)
        .map_err(|_| {
            msg!("Withdraw: Failed to unpack user token account");
            VaultError::InvalidTokenAccount
        })?;
    
    if user_token_account.mint != vault_state.token_mint {
        msg!("Withdraw: User token account mint mismatch");
        return Err(VaultError::InvalidMint.into());
    }
    drop(user_token_data);

    // Verify vault token account
    let vault_token_data = vault_token_account_info.try_borrow_data()?;
    let vault_token_account = spl_token::state::Account::unpack(&vault_token_data)
        .map_err(|_| {
            msg!("Withdraw: Failed to unpack vault token account");
            VaultError::InvalidTokenAccount
        })?;
    
    if vault_token_account.mint != vault_state.token_mint {
        msg!("Withdraw: Vault token account mint mismatch");
        return Err(VaultError::InvalidMint.into());
    }

    // Verify vault has sufficient tokens
    if vault_token_account.amount < amount {
        msg!("Withdraw: Insufficient vault token balance. Required: {}, Available: {}", 
             amount, vault_token_account.amount);
        return Err(VaultError::InsufficientFunds.into());
    }
    drop(vault_token_data);

    // Derive and verify user balance PDA
    let (user_balance_pda, user_balance_bump) = derive_user_balance_pda(
        program_id,
        user_info.key,
        vault_state_info.key,
    )?;

    if user_balance_pda != *user_balance_info.key {
        msg!("Withdraw: User balance PDA mismatch. Expected: {}, Got: {}", 
             user_balance_pda, user_balance_info.key);
        return Err(VaultError::InvalidInput.into());
    }

    // Load user balance account
    if user_balance_info.owner != program_id {
        msg!("Withdraw: User balance account not owned by program");
        return Err(VaultError::InvalidInput.into());
    }

    let mut user_balance_data = user_balance_info.try_borrow_mut_data()?;
    let mut user_balance = deserialize_user_balance_safe(&user_balance_data, "Withdraw")?;

    // Validate user balance account
    user_balance.validate().map_err(|err| {
        msg!("Withdraw: User balance validation failed: {}", err);
        VaultError::InvalidInput
    })?;

    // Check if user has sufficient balance
    if !user_balance.has_sufficient_balance(amount) {
        msg!("Withdraw: Insufficient user balance. Required: {}, Available: {}", 
             amount, user_balance.balance);
        return Err(VaultError::InsufficientFunds.into());
    }

    // Create transfer instruction from vault to user
    let transfer_ix = spl_token::instruction::transfer(
        &spl_token::id(),
        vault_token_account_info.key,
        user_token_account_info.key,
        vault_state_info.key, // Vault state account is the authority
        &[],
        amount,
    )?;

    // Create vault state seeds for signing
    let vault_state_seeds = &[
        crate::utils::VAULT_SEED,
        vault_state.owner.as_ref(),
        vault_state.token_mint.as_ref(),
        &[vault_state.bump],
    ];

    // Execute the transfer with vault state as signer
    invoke_signed(
        &transfer_ix,
        &[
            vault_token_account_info.clone(),
            user_token_account_info.clone(),
            vault_state_info.clone(),
            token_program_info.clone(),
        ],
        &[vault_state_seeds],
    ).map_err(|e| {
        msg!("Withdraw: Token transfer failed: {}", e);
        e
    })?;

    // Update user balance with underflow protection
    user_balance.subtract_balance(amount).map_err(|err| {
        msg!("Withdraw: Failed to update user balance: {}", err);
        VaultError::ArithmeticOverflow
    })?;

    // Update vault total deposited with underflow protection
    vault_state.subtract_withdrawal(amount).map_err(|err| {
        msg!("Withdraw: Failed to update vault total: {}", err);
        VaultError::ArithmeticOverflow
    })?;

    // Save updated user balance
    serialize_user_balance_safe(&user_balance, &mut *user_balance_data, "Withdraw")?;
    drop(user_balance_data);

    // Save updated vault state
    let mut vault_state_data = vault_state_info.try_borrow_mut_data()?;
    serialize_vault_state(&vault_state, &mut *vault_state_data, "Withdraw")?;

    msg!(
        "Withdraw successful. User: {}, Amount: {}, New Balance: {}, Vault Total: {}",
        user_info.key,
        amount,
        user_balance.balance,
        vault_state.total_deposited
    );

    Ok(())
}

/// Process WithdrawAll instruction
/// Allows vault owner to withdraw all funds from the vault
pub fn process_withdraw_all(program_id: &Pubkey, accounts: &[AccountInfo]) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();
    
    // Validate minimum number of accounts
    if accounts.len() < 5 {
        msg!("WithdrawAll: Insufficient accounts provided");
        return Err(VaultError::InvalidInput.into());
    }
    
    // Expected accounts:
    // 0. [signer, writable] Vault owner
    // 1. [writable] Owner token account
    // 2. [writable] Vault token account
    // 3. [writable] Vault state account
    // 4. [] SPL Token program
    let owner_info = next_account_info(account_info_iter)?;
    let owner_token_account_info = next_account_info(account_info_iter)?;
    let vault_token_account_info = next_account_info(account_info_iter)?;
    let vault_state_info = next_account_info(account_info_iter)?;
    let token_program_info = next_account_info(account_info_iter)?;

    // Verify owner is signer
    if !owner_info.is_signer {
        msg!("WithdrawAll: Owner must be signer");
        return Err(VaultError::UnauthorizedAccess.into());
    }

    // Verify accounts are writable
    if !owner_token_account_info.is_writable {
        msg!("WithdrawAll: Owner token account must be writable");
        return Err(VaultError::InvalidInput.into());
    }
    if !vault_token_account_info.is_writable {
        msg!("WithdrawAll: Vault token account must be writable");
        return Err(VaultError::InvalidInput.into());
    }
    if !vault_state_info.is_writable {
        msg!("WithdrawAll: Vault state account must be writable");
        return Err(VaultError::InvalidInput.into());
    }

    // Verify program accounts
    if token_program_info.key != &spl_token::id() {
        msg!("WithdrawAll: Invalid SPL Token program");
        return Err(VaultError::InvalidTokenAccount.into());
    }

    // Load and validate vault state
    let vault_state_data = vault_state_info.try_borrow_data()?;
    let mut vault_state = deserialize_vault_state_safe(&vault_state_data, "WithdrawAll")?;
    drop(vault_state_data); // Drop the read borrow early

    // Check if vault is operational
    if !vault_state.is_operational() {
        msg!("WithdrawAll: Vault is closed");
        return Err(VaultError::VaultClosed.into());
    }

    // Verify vault state account ownership
    if vault_state_info.owner != program_id {
        msg!("WithdrawAll: Vault state account not owned by program");
        return Err(VaultError::InvalidInput.into());
    }

    // Verify caller is the vault owner
    if *owner_info.key != vault_state.owner {
        msg!("WithdrawAll: Caller is not the vault owner. Expected: {}, Got: {}", 
             vault_state.owner, owner_info.key);
        return Err(VaultError::UnauthorizedAccess.into());
    }

    // Verify token accounts are owned by SPL Token program
    if owner_token_account_info.owner != &spl_token::id() {
        msg!("WithdrawAll: Owner token account must be owned by SPL Token program");
        return Err(VaultError::InvalidTokenAccount.into());
    }
    if vault_token_account_info.owner != &spl_token::id() {
        msg!("WithdrawAll: Vault token account must be owned by SPL Token program");
        return Err(VaultError::InvalidTokenAccount.into());
    }

    // Verify token accounts match the vault's mint
    let owner_token_data = owner_token_account_info.try_borrow_data()?;
    let owner_token_account = spl_token::state::Account::unpack(&owner_token_data)
        .map_err(|_| {
            msg!("WithdrawAll: Failed to unpack owner token account");
            VaultError::InvalidTokenAccount
        })?;
    
    if owner_token_account.mint != vault_state.token_mint {
        msg!("WithdrawAll: Owner token account mint mismatch");
        return Err(VaultError::InvalidMint.into());
    }
    drop(owner_token_data);

    // Get vault token account balance
    let vault_token_data = vault_token_account_info.try_borrow_data()?;
    let vault_token_account = spl_token::state::Account::unpack(&vault_token_data)
        .map_err(|_| {
            msg!("WithdrawAll: Failed to unpack vault token account");
            VaultError::InvalidTokenAccount
        })?;
    
    if vault_token_account.mint != vault_state.token_mint {
        msg!("WithdrawAll: Vault token account mint mismatch");
        return Err(VaultError::InvalidMint.into());
    }

    let total_amount = vault_token_account.amount;
    drop(vault_token_data);

    // Check if there are any tokens to withdraw
    if total_amount == 0 {
        msg!("WithdrawAll: No tokens to withdraw");
        return Ok(());
    }

    // Create transfer instruction from vault to owner
    let transfer_ix = spl_token::instruction::transfer(
        &spl_token::id(),
        vault_token_account_info.key,
        owner_token_account_info.key,
        vault_state_info.key, // Vault state account is the authority
        &[],
        total_amount,
    )?;

    // Create vault state seeds for signing
    let vault_state_seeds = &[
        crate::utils::VAULT_SEED,
        vault_state.owner.as_ref(),
        vault_state.token_mint.as_ref(),
        &[vault_state.bump],
    ];

    // Execute the transfer with vault state as signer
    invoke_signed(
        &transfer_ix,
        &[
            vault_token_account_info.clone(),
            owner_token_account_info.clone(),
            vault_state_info.clone(),
            token_program_info.clone(),
        ],
        &[vault_state_seeds],
    ).map_err(|e| {
        msg!("WithdrawAll: Token transfer failed: {}", e);
        e
    })?;

    // Reset vault total deposited to zero
    vault_state.reset_total_deposited();

    // Save updated vault state
    let mut vault_state_data = vault_state_info.try_borrow_mut_data()?;
    serialize_vault_state(&vault_state, &mut *vault_state_data, "WithdrawAll")?;

    msg!(
        "WithdrawAll successful. Owner: {}, Amount: {}, Vault Total Reset: {}",
        owner_info.key,
        total_amount,
        vault_state.total_deposited
    );

    Ok(())
}

/// Process Close instruction
/// Allows vault owner to close the vault and transfer any remaining tokens
pub fn process_close(program_id: &Pubkey, accounts: &[AccountInfo]) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();
    
    // Validate minimum number of accounts
    if accounts.len() < 5 {
        msg!("Close: Insufficient accounts provided");
        return Err(VaultError::InvalidInput.into());
    }
    
    // Expected accounts:
    // 0. [signer, writable] Vault owner
    // 1. [writable] Owner token account (to receive remaining tokens)
    // 2. [writable] Vault token account
    // 3. [writable] Vault state account
    // 4. [] SPL Token program
    let owner_info = next_account_info(account_info_iter)?;
    let owner_token_account_info = next_account_info(account_info_iter)?;
    let vault_token_account_info = next_account_info(account_info_iter)?;
    let vault_state_info = next_account_info(account_info_iter)?;
    let token_program_info = next_account_info(account_info_iter)?;

    // Verify owner is signer
    if !owner_info.is_signer {
        msg!("Close: Owner must be signer");
        return Err(VaultError::UnauthorizedAccess.into());
    }

    // Verify accounts are writable
    if !owner_token_account_info.is_writable {
        msg!("Close: Owner token account must be writable");
        return Err(VaultError::InvalidInput.into());
    }
    if !vault_token_account_info.is_writable {
        msg!("Close: Vault token account must be writable");
        return Err(VaultError::InvalidInput.into());
    }
    if !vault_state_info.is_writable {
        msg!("Close: Vault state account must be writable");
        return Err(VaultError::InvalidInput.into());
    }

    // Verify program accounts
    if token_program_info.key != &spl_token::id() {
        msg!("Close: Invalid SPL Token program");
        return Err(VaultError::InvalidTokenAccount.into());
    }

    // Load and validate vault state
    let vault_state_data = vault_state_info.try_borrow_data()?;
    let mut vault_state = deserialize_vault_state_safe(&vault_state_data, "Close")?;
    drop(vault_state_data); // Drop the read borrow early

    // Check if vault is already closed
    if !vault_state.is_operational() {
        msg!("Close: Vault is already closed");
        return Err(VaultError::VaultClosed.into());
    }

    // Verify vault state account ownership
    if vault_state_info.owner != program_id {
        msg!("Close: Vault state account not owned by program");
        return Err(VaultError::InvalidInput.into());
    }

    // Verify caller is the vault owner
    if *owner_info.key != vault_state.owner {
        msg!("Close: Caller is not the vault owner. Expected: {}, Got: {}", 
             vault_state.owner, owner_info.key);
        return Err(VaultError::UnauthorizedAccess.into());
    }

    // Verify token accounts are owned by SPL Token program
    if owner_token_account_info.owner != &spl_token::id() {
        msg!("Close: Owner token account must be owned by SPL Token program");
        return Err(VaultError::InvalidTokenAccount.into());
    }
    if vault_token_account_info.owner != &spl_token::id() {
        msg!("Close: Vault token account must be owned by SPL Token program");
        return Err(VaultError::InvalidTokenAccount.into());
    }

    // Verify token accounts match the vault's mint
    let owner_token_data = owner_token_account_info.try_borrow_data()?;
    let owner_token_account = spl_token::state::Account::unpack(&owner_token_data)
        .map_err(|_| {
            msg!("Close: Failed to unpack owner token account");
            VaultError::InvalidTokenAccount
        })?;
    
    if owner_token_account.mint != vault_state.token_mint {
        msg!("Close: Owner token account mint mismatch");
        return Err(VaultError::InvalidMint.into());
    }
    drop(owner_token_data);

    // Get vault token account balance
    let vault_token_data = vault_token_account_info.try_borrow_data()?;
    let vault_token_account = spl_token::state::Account::unpack(&vault_token_data)
        .map_err(|_| {
            msg!("Close: Failed to unpack vault token account");
            VaultError::InvalidTokenAccount
        })?;
    
    if vault_token_account.mint != vault_state.token_mint {
        msg!("Close: Vault token account mint mismatch");
        return Err(VaultError::InvalidMint.into());
    }

    let remaining_amount = vault_token_account.amount;
    drop(vault_token_data);

    // Transfer any remaining tokens to owner before closing
    if remaining_amount > 0 {
        let transfer_ix = spl_token::instruction::transfer(
            &spl_token::id(),
            vault_token_account_info.key,
            owner_token_account_info.key,
            vault_state_info.key, // Vault state account is the authority
            &[],
            remaining_amount,
        )?;

        // Create vault state seeds for signing
        let vault_state_seeds = &[
            crate::utils::VAULT_SEED,
            vault_state.owner.as_ref(),
            vault_state.token_mint.as_ref(),
            &[vault_state.bump],
        ];

        // Execute the transfer with vault state as signer
        invoke_signed(
            &transfer_ix,
            &[
                vault_token_account_info.clone(),
                owner_token_account_info.clone(),
                vault_state_info.clone(),
                token_program_info.clone(),
            ],
            &[vault_state_seeds],
        ).map_err(|e| {
            msg!("Close: Token transfer failed: {}", e);
            e
        })?;
    }

    // Mark vault as closed
    vault_state.close();

    // Save updated vault state
    let mut vault_state_data = vault_state_info.try_borrow_mut_data()?;
    serialize_vault_state(&vault_state, &mut *vault_state_data, "Close")?;

    msg!(
        "Vault closed successfully. Owner: {}, Remaining tokens transferred: {}, Vault is now closed",
        owner_info.key,
        remaining_amount
    );

    Ok(())
}
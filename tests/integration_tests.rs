use borsh::BorshDeserialize;
use solana_program::{
    program_pack::Pack,
    pubkey::Pubkey,
    system_instruction,
};
use solana_program_test::*;
use solana_sdk::{
    signature::{Keypair, Signer},
    transaction::Transaction,
};
use spl_token::state::{Account as TokenAccount, Mint};

use solana_vault_contract::{
    instruction::VaultInstruction,
    state::{VaultState, UserBalance},
    utils::{derive_vault_state_pda, derive_user_balance_pda},
};

/// Test context containing all necessary accounts and keypairs
pub struct TestContext {
    pub program_id: Pubkey,
    pub owner: Keypair,
    pub user1: Keypair,
    pub user2: Keypair,
    pub token_mint: Keypair,
    pub vault_token_account: Keypair,
    pub owner_token_account: Keypair,
    pub user1_token_account: Keypair,
    pub user2_token_account: Keypair,
    pub vault_state_pda: Pubkey,
    pub vault_state_bump: u8,
    pub user1_balance_pda: Pubkey,
    pub user1_balance_bump: u8,
    pub user2_balance_pda: Pubkey,
    pub user2_balance_bump: u8,
}

impl TestContext {
    pub fn new() -> Self {
        let program_id = solana_vault_contract::id(); // Use the actual program ID
        let owner = Keypair::new();
        let user1 = Keypair::new();
        let user2 = Keypair::new();
        let token_mint = Keypair::new();
        let vault_token_account = Keypair::new();
        let owner_token_account = Keypair::new();
        let user1_token_account = Keypair::new();
        let user2_token_account = Keypair::new();

        let (vault_state_pda, vault_state_bump) = derive_vault_state_pda(
            &program_id,
            &owner.pubkey(),
            &token_mint.pubkey(),
        ).unwrap();

        let (user1_balance_pda, user1_balance_bump) = derive_user_balance_pda(
            &program_id,
            &user1.pubkey(),
            &vault_state_pda,
        ).unwrap();

        let (user2_balance_pda, user2_balance_bump) = derive_user_balance_pda(
            &program_id,
            &user2.pubkey(),
            &vault_state_pda,
        ).unwrap();

        Self {
            program_id,
            owner,
            user1,
            user2,
            token_mint,
            vault_token_account,
            owner_token_account,
            user1_token_account,
            user2_token_account,
            vault_state_pda,
            vault_state_bump,
            user1_balance_pda,
            user1_balance_bump,
            user2_balance_pda,
            user2_balance_bump,
        }
    }

    /// Helper function to recalculate PDAs when owner changes
    pub fn recalculate_pdas_for_owner(&mut self, new_owner: Keypair) {
        self.owner = new_owner;
        
        // Recalculate vault state PDA with new owner
        let (vault_state_pda, vault_state_bump) = derive_vault_state_pda(
            &self.program_id,
            &self.owner.pubkey(),
            &self.token_mint.pubkey(),
        ).unwrap();
        self.vault_state_pda = vault_state_pda;
        self.vault_state_bump = vault_state_bump;
        
        // Recalculate user balance PDAs with new vault state PDA
        let (user1_balance_pda, user1_balance_bump) = derive_user_balance_pda(
            &self.program_id,
            &self.user1.pubkey(),
            &self.vault_state_pda,
        ).unwrap();
        self.user1_balance_pda = user1_balance_pda;
        self.user1_balance_bump = user1_balance_bump;
        
        let (user2_balance_pda, user2_balance_bump) = derive_user_balance_pda(
            &self.program_id,
            &self.user2.pubkey(),
            &self.vault_state_pda,
        ).unwrap();
        self.user2_balance_pda = user2_balance_pda;
        self.user2_balance_bump = user2_balance_bump;
    }
}

/// Create a test program context with the vault program
pub fn create_program_test() -> ProgramTest {
    let mut program_test = ProgramTest::new(
        "solana_vault_contract",
        solana_vault_contract::id(),
        processor!(solana_vault_contract::process_instruction),
    );

    // Configure to use native programs instead of BPF
    program_test.prefer_bpf(false);
    
    program_test
}

/// Setup token mint and accounts for testing
pub async fn setup_token_accounts(
    banks_client: &mut BanksClient,
    payer: &Keypair,
    context: &TestContext,
    initial_supply: u64,
) -> Result<(), Box<dyn std::error::Error>> {
    let rent = banks_client.get_rent().await?;
    
    // Fund user accounts with enough lamports for account creation
    let user_funding_amount = 10_000_000; // 0.01 SOL should be enough
    
    let fund_user1_ix = system_instruction::transfer(
        &payer.pubkey(),
        &context.user1.pubkey(),
        user_funding_amount,
    );
    
    let fund_user2_ix = system_instruction::transfer(
        &payer.pubkey(),
        &context.user2.pubkey(),
        user_funding_amount,
    );
    
    let mut transaction = Transaction::new_with_payer(
        &[fund_user1_ix, fund_user2_ix],
        Some(&payer.pubkey()),
    );
    let recent_blockhash = banks_client.get_latest_blockhash().await?;
    transaction.sign(&[payer], recent_blockhash);
    banks_client.process_transaction(transaction).await?;

    // Create token mint
    let mint_rent = rent.minimum_balance(Mint::LEN);
    let create_mint_ix = system_instruction::create_account(
        &payer.pubkey(),
        &context.token_mint.pubkey(),
        mint_rent,
        Mint::LEN as u64,
        &spl_token::id(),
    );

    let initialize_mint_ix = spl_token::instruction::initialize_mint(
        &spl_token::id(),
        &context.token_mint.pubkey(),
        &payer.pubkey(),
        None,
        6, // 6 decimals
    )?;

    let mut transaction = Transaction::new_with_payer(
        &[create_mint_ix, initialize_mint_ix],
        Some(&payer.pubkey()),
    );
    let recent_blockhash = banks_client.get_latest_blockhash().await?;
    transaction.sign(&[payer, &context.token_mint], recent_blockhash);
    banks_client.process_transaction(transaction).await?;

    // Create token accounts
    let token_account_rent = rent.minimum_balance(TokenAccount::LEN);
    
    // Vault token account
    let create_vault_token_ix = system_instruction::create_account(
        &payer.pubkey(),
        &context.vault_token_account.pubkey(),
        token_account_rent,
        TokenAccount::LEN as u64,
        &spl_token::id(),
    );
    let init_vault_token_ix = spl_token::instruction::initialize_account(
        &spl_token::id(),
        &context.vault_token_account.pubkey(),
        &context.token_mint.pubkey(),
        &context.vault_state_pda,
    )?;

    // Owner token account
    let create_owner_token_ix = system_instruction::create_account(
        &payer.pubkey(),
        &context.owner_token_account.pubkey(),
        token_account_rent,
        TokenAccount::LEN as u64,
        &spl_token::id(),
    );
    let init_owner_token_ix = spl_token::instruction::initialize_account(
        &spl_token::id(),
        &context.owner_token_account.pubkey(),
        &context.token_mint.pubkey(),
        &context.owner.pubkey(),
    )?;

    // User1 token account
    let create_user1_token_ix = system_instruction::create_account(
        &payer.pubkey(),
        &context.user1_token_account.pubkey(),
        token_account_rent,
        TokenAccount::LEN as u64,
        &spl_token::id(),
    );
    let init_user1_token_ix = spl_token::instruction::initialize_account(
        &spl_token::id(),
        &context.user1_token_account.pubkey(),
        &context.token_mint.pubkey(),
        &context.user1.pubkey(),
    )?;

    // User2 token account
    let create_user2_token_ix = system_instruction::create_account(
        &payer.pubkey(),
        &context.user2_token_account.pubkey(),
        token_account_rent,
        TokenAccount::LEN as u64,
        &spl_token::id(),
    );
    let init_user2_token_ix = spl_token::instruction::initialize_account(
        &spl_token::id(),
        &context.user2_token_account.pubkey(),
        &context.token_mint.pubkey(),
        &context.user2.pubkey(),
    )?;

    let mut transaction = Transaction::new_with_payer(
        &[
            create_vault_token_ix,
            init_vault_token_ix,
            create_owner_token_ix,
            init_owner_token_ix,
            create_user1_token_ix,
            init_user1_token_ix,
            create_user2_token_ix,
            init_user2_token_ix,
        ],
        Some(&payer.pubkey()),
    );
    let recent_blockhash = banks_client.get_latest_blockhash().await?;
    let signers: Vec<&dyn Signer> = vec![
        payer,
        &context.vault_token_account,
        &context.owner_token_account,
        &context.user1_token_account,
        &context.user2_token_account,
    ];
    transaction.sign(&signers, recent_blockhash);
    banks_client.process_transaction(transaction).await?;

    // Mint tokens to users for testing
    if initial_supply > 0 {
        let mint_to_user1_ix = spl_token::instruction::mint_to(
            &spl_token::id(),
            &context.token_mint.pubkey(),
            &context.user1_token_account.pubkey(),
            &payer.pubkey(),
            &[],
            initial_supply,
        )?;

        let mint_to_user2_ix = spl_token::instruction::mint_to(
            &spl_token::id(),
            &context.token_mint.pubkey(),
            &context.user2_token_account.pubkey(),
            &payer.pubkey(),
            &[],
            initial_supply,
        )?;

        let mut transaction = Transaction::new_with_payer(
            &[mint_to_user1_ix, mint_to_user2_ix],
            Some(&payer.pubkey()),
        );
        let recent_blockhash = banks_client.get_latest_blockhash().await?;
        transaction.sign(&[payer], recent_blockhash);
        banks_client.process_transaction(transaction).await?;
    }

    Ok(())
}

/// Helper function to get token account balance
pub async fn get_token_balance(
    banks_client: &mut BanksClient,
    token_account: &Pubkey,
) -> Result<u64, Box<dyn std::error::Error>> {
    let account = banks_client.get_account(*token_account).await?.unwrap();
    let token_account_data = TokenAccount::unpack(&account.data)?;
    Ok(token_account_data.amount)
}

/// Helper function to get vault state
pub async fn get_vault_state(
    banks_client: &mut BanksClient,
    vault_state_pda: &Pubkey,
) -> Result<VaultState, Box<dyn std::error::Error>> {
    let account = banks_client.get_account(*vault_state_pda).await?.unwrap();
    println!("Account data length: {}", account.data.len());
    println!("Expected VaultState size: {}", VaultState::SIZE);
    println!("Account owner: {}", account.owner);
    println!("Account lamports: {}", account.lamports);
    println!("Account executable: {}", account.executable);
    
    if account.data.len() == 0 {
        return Err("Account has no data".into());
    }
    
    // Debug: Print first few bytes of the account data
    println!("First 20 bytes of account data: {:?}", &account.data[..20.min(account.data.len())]);
    
    // Try to deserialize with enhanced error reporting
    println!("Attempting to deserialize vault state...");
    println!("Account data length: {}", account.data.len());
    println!("Expected VaultState size: {}", 106); // VaultState::SIZE not accessible here
    
    if account.data.len() != 106 {
        println!("Buffer size mismatch - expected: 106, actual: {}", account.data.len());
        return Err("Buffer size mismatch".into());
    }
    
    let vault_state = VaultState::try_from_slice(&account.data)
        .map_err(|e| {
            println!("Deserialization failed: {}", e);
            println!("Account data (hex): {}", hex::encode(&account.data));
            println!("First 20 bytes: {:?}", &account.data[..20.min(account.data.len())]);
            if account.data.len() > 20 {
                let tail_start = account.data.len().saturating_sub(20);
                println!("Last 20 bytes: {:?}", &account.data[tail_start..]);
            }
            e
        })?;
    
    println!("Successfully deserialized vault state");
    Ok(vault_state)
}

/// Helper function to get user balance
pub async fn get_user_balance(
    banks_client: &mut BanksClient,
    user_balance_pda: &Pubkey,
) -> Result<Option<UserBalance>, Box<dyn std::error::Error>> {
    match banks_client.get_account(*user_balance_pda).await? {
        Some(account) => {
            println!("Attempting to deserialize user balance...");
            println!("User balance account data length: {}", account.data.len());
            println!("Expected UserBalance size: {}", 73); // UserBalance::SIZE = 32 + 32 + 8 + 1 = 73
            
            if account.data.len() != 73 {
                println!("User balance buffer size mismatch - expected: 73, actual: {}", account.data.len());
                return Err("User balance buffer size mismatch".into());
            }
            
            let user_balance = UserBalance::try_from_slice(&account.data)
                .map_err(|e| {
                    println!("User balance deserialization failed: {}", e);
                    println!("User balance account data (hex): {}", hex::encode(&account.data));
                    println!("First 20 bytes: {:?}", &account.data[..20.min(account.data.len())]);
                    if account.data.len() > 20 {
                        let tail_start = account.data.len().saturating_sub(20);
                        println!("Last 20 bytes: {:?}", &account.data[tail_start..]);
                    }
                    e
                })?;
            
            println!("Successfully deserialized user balance");
            Ok(Some(user_balance))
        }
        None => Ok(None),
    }
}

#[tokio::test]
async fn test_initialize_vault() {
    let program_test = create_program_test();
    let (mut banks_client, payer, recent_blockhash) = program_test.start().await;
    
    // Create a new context but use payer as owner to simplify funding
    let mut context = TestContext::new();
    context.recalculate_pdas_for_owner(Keypair::from_bytes(&payer.to_bytes()).unwrap());

    // Setup token accounts
    setup_token_accounts(&mut banks_client, &payer, &context, 1000000).await.unwrap();

    // Initialize vault
    let initialize_ix = VaultInstruction::initialize(
        &context.program_id,
        &context.owner.pubkey(),
        &context.vault_state_pda,
        &context.vault_token_account.pubkey(),
        &context.token_mint.pubkey(),
    );

    println!("Program ID: {}", context.program_id);
    println!("Owner: {}", context.owner.pubkey());
    println!("Vault State PDA: {}", context.vault_state_pda);
    println!("Instruction program_id: {}", initialize_ix.program_id);
    println!("Instruction data length: {}", initialize_ix.data.len());

    let mut transaction = Transaction::new_with_payer(&[initialize_ix], Some(&payer.pubkey()));
    transaction.sign(&[&payer], recent_blockhash); // Only need payer since owner is payer
    
    let result = banks_client.process_transaction(transaction).await;
    if let Err(e) = &result {
        println!("Vault initialization failed: {:?}", e);
    }
    assert!(result.is_ok(), "Vault initialization should succeed");

    // Wait a bit for the transaction to be fully processed
    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

    // Verify vault state
    let vault_state = get_vault_state(&mut banks_client, &context.vault_state_pda).await.unwrap();
    assert_eq!(vault_state.owner, context.owner.pubkey());
    assert_eq!(vault_state.token_mint, context.token_mint.pubkey());
    assert_eq!(vault_state.token_account, context.vault_token_account.pubkey());
    assert_eq!(vault_state.total_deposited, 0);
    assert!(!vault_state.is_closed());
}

#[tokio::test]
async fn test_deposit_tokens() {
    let program_test = create_program_test();
    let (mut banks_client, payer, recent_blockhash) = program_test.start().await;
    
    // Create a new context but use payer as owner to simplify funding
    let mut context = TestContext::new();
    context.recalculate_pdas_for_owner(Keypair::from_bytes(&payer.to_bytes()).unwrap());

    // Setup and initialize vault
    setup_token_accounts(&mut banks_client, &payer, &context, 1000000).await.unwrap();
    
    let initialize_ix = VaultInstruction::initialize(
        &context.program_id,
        &context.owner.pubkey(),
        &context.vault_state_pda,
        &context.vault_token_account.pubkey(),
        &context.token_mint.pubkey(),
    );

    let mut transaction = Transaction::new_with_payer(&[initialize_ix], Some(&payer.pubkey()));
    transaction.sign(&[&payer], recent_blockhash); // Only need payer since owner is payer
    banks_client.process_transaction(transaction).await.unwrap();

    // Check payer balance before deposit
    let payer_account = banks_client.get_account(payer.pubkey()).await.unwrap().unwrap();
    println!("Payer lamports before deposit: {}", payer_account.lamports);

    // Test deposit
    let deposit_amount = 100000;
    let deposit_ix = VaultInstruction::deposit(
        &context.program_id,
        &context.user1.pubkey(),
        &context.user1_token_account.pubkey(),
        &context.vault_token_account.pubkey(),
        &context.vault_state_pda,
        &context.user1_balance_pda,
        deposit_amount,
    );

    let mut transaction = Transaction::new_with_payer(&[deposit_ix], Some(&payer.pubkey()));
    let recent_blockhash = banks_client.get_latest_blockhash().await.unwrap();
    transaction.sign(&[&payer, &context.user1], recent_blockhash);
    
    let result = banks_client.process_transaction(transaction).await;
    if let Err(e) = &result {
        println!("Deposit failed: {:?}", e);
    }
    assert!(result.is_ok(), "Deposit should succeed");

    // Wait a bit for the transaction to be fully processed
    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

    // Debug: Print the vault state PDA we're trying to read
    println!("Trying to read vault state from PDA: {}", context.vault_state_pda);
    
    // Verify balances
    let user1_token_balance = get_token_balance(&mut banks_client, &context.user1_token_account.pubkey()).await.unwrap();
    let vault_token_balance = get_token_balance(&mut banks_client, &context.vault_token_account.pubkey()).await.unwrap();
    let vault_state = get_vault_state(&mut banks_client, &context.vault_state_pda).await.unwrap();
    let user_balance = get_user_balance(&mut banks_client, &context.user1_balance_pda).await.unwrap().unwrap();

    assert_eq!(user1_token_balance, 1000000 - deposit_amount);
    assert_eq!(vault_token_balance, deposit_amount);
    assert_eq!(vault_state.total_deposited, deposit_amount);
    assert_eq!(user_balance.balance, deposit_amount);
}

#[tokio::test]
async fn test_withdraw_tokens() {
    let program_test = create_program_test();
    let (mut banks_client, payer, recent_blockhash) = program_test.start().await;
    
    // Create a new context but use payer as owner to simplify funding
    let mut context = TestContext::new();
    context.recalculate_pdas_for_owner(Keypair::from_bytes(&payer.to_bytes()).unwrap());

    // Setup, initialize vault, and deposit tokens
    setup_token_accounts(&mut banks_client, &payer, &context, 1000000).await.unwrap();
    
    let initialize_ix = VaultInstruction::initialize(
        &context.program_id,
        &context.owner.pubkey(),
        &context.vault_state_pda,
        &context.vault_token_account.pubkey(),
        &context.token_mint.pubkey(),
    );

    let mut transaction = Transaction::new_with_payer(&[initialize_ix], Some(&payer.pubkey()));
    transaction.sign(&[&payer], recent_blockhash); // Only need payer since owner is payer
    banks_client.process_transaction(transaction).await.unwrap();

    // Deposit first
    let deposit_amount = 100000;
    let deposit_ix = VaultInstruction::deposit(
        &context.program_id,
        &context.user1.pubkey(),
        &context.user1_token_account.pubkey(),
        &context.vault_token_account.pubkey(),
        &context.vault_state_pda,
        &context.user1_balance_pda,
        deposit_amount,
    );

    let mut transaction = Transaction::new_with_payer(&[deposit_ix], Some(&payer.pubkey()));
    let recent_blockhash = banks_client.get_latest_blockhash().await.unwrap();
    transaction.sign(&[&payer, &context.user1], recent_blockhash);
    banks_client.process_transaction(transaction).await.unwrap();

    // Test withdrawal
    let withdraw_amount = 50000;
    let withdraw_ix = VaultInstruction::withdraw(
        &context.program_id,
        &context.user1.pubkey(),
        &context.user1_token_account.pubkey(),
        &context.vault_token_account.pubkey(),
        &context.vault_state_pda,
        &context.user1_balance_pda,
        withdraw_amount,
    );

    let mut transaction = Transaction::new_with_payer(&[withdraw_ix], Some(&payer.pubkey()));
    let recent_blockhash = banks_client.get_latest_blockhash().await.unwrap();
    transaction.sign(&[&payer, &context.user1], recent_blockhash);
    
    let result = banks_client.process_transaction(transaction).await;
    assert!(result.is_ok(), "Withdrawal should succeed");

    // Verify balances
    let user1_token_balance = get_token_balance(&mut banks_client, &context.user1_token_account.pubkey()).await.unwrap();
    let vault_token_balance = get_token_balance(&mut banks_client, &context.vault_token_account.pubkey()).await.unwrap();
    let vault_state = get_vault_state(&mut banks_client, &context.vault_state_pda).await.unwrap();
    let user_balance = get_user_balance(&mut banks_client, &context.user1_balance_pda).await.unwrap().unwrap();

    assert_eq!(user1_token_balance, 1000000 - deposit_amount + withdraw_amount);
    assert_eq!(vault_token_balance, deposit_amount - withdraw_amount);
    assert_eq!(vault_state.total_deposited, deposit_amount - withdraw_amount);
    assert_eq!(user_balance.balance, deposit_amount - withdraw_amount);
}

#[tokio::test]
async fn test_insufficient_funds_withdrawal() {
    let program_test = create_program_test();
    let (mut banks_client, payer, recent_blockhash) = program_test.start().await;
    
    // Create a new context but use payer as owner to simplify funding
    let mut context = TestContext::new();
    context.recalculate_pdas_for_owner(Keypair::from_bytes(&payer.to_bytes()).unwrap());

    // Setup, initialize vault, and deposit tokens
    setup_token_accounts(&mut banks_client, &payer, &context, 1000000).await.unwrap();
    
    let initialize_ix = VaultInstruction::initialize(
        &context.program_id,
        &context.owner.pubkey(),
        &context.vault_state_pda,
        &context.vault_token_account.pubkey(),
        &context.token_mint.pubkey(),
    );

    let mut transaction = Transaction::new_with_payer(&[initialize_ix], Some(&payer.pubkey()));
    transaction.sign(&[&payer], recent_blockhash); // Only need payer since owner is payer
    banks_client.process_transaction(transaction).await.unwrap();

    // Deposit first
    let deposit_amount = 100000;
    let deposit_ix = VaultInstruction::deposit(
        &context.program_id,
        &context.user1.pubkey(),
        &context.user1_token_account.pubkey(),
        &context.vault_token_account.pubkey(),
        &context.vault_state_pda,
        &context.user1_balance_pda,
        deposit_amount,
    );

    let mut transaction = Transaction::new_with_payer(&[deposit_ix], Some(&payer.pubkey()));
    let recent_blockhash = banks_client.get_latest_blockhash().await.unwrap();
    transaction.sign(&[&payer, &context.user1], recent_blockhash);
    banks_client.process_transaction(transaction).await.unwrap();

    // Try to withdraw more than deposited
    let withdraw_amount = 200000; // More than deposited
    let withdraw_ix = VaultInstruction::withdraw(
        &context.program_id,
        &context.user1.pubkey(),
        &context.user1_token_account.pubkey(),
        &context.vault_token_account.pubkey(),
        &context.vault_state_pda,
        &context.user1_balance_pda,
        withdraw_amount,
    );

    let mut transaction = Transaction::new_with_payer(&[withdraw_ix], Some(&payer.pubkey()));
    let recent_blockhash = banks_client.get_latest_blockhash().await.unwrap();
    transaction.sign(&[&payer, &context.user1], recent_blockhash);
    
    let result = banks_client.process_transaction(transaction).await;
    assert!(result.is_err(), "Withdrawal should fail due to insufficient funds");
}

#[tokio::test]
async fn test_owner_withdraw_all() {
    let program_test = create_program_test();
    let (mut banks_client, payer, recent_blockhash) = program_test.start().await;
    
    // Create a new context but use payer as owner to simplify funding
    let mut context = TestContext::new();
    context.recalculate_pdas_for_owner(Keypair::from_bytes(&payer.to_bytes()).unwrap());

    // Setup, initialize vault, and deposit tokens from multiple users
    setup_token_accounts(&mut banks_client, &payer, &context, 1000000).await.unwrap();
    
    let initialize_ix = VaultInstruction::initialize(
        &context.program_id,
        &context.owner.pubkey(),
        &context.vault_state_pda,
        &context.vault_token_account.pubkey(),
        &context.token_mint.pubkey(),
    );

    let mut transaction = Transaction::new_with_payer(&[initialize_ix], Some(&payer.pubkey()));
    transaction.sign(&[&payer], recent_blockhash); // Only need payer since owner is payer
    banks_client.process_transaction(transaction).await.unwrap();

    // Deposit from user1
    let deposit_amount1 = 100000;
    let deposit_ix1 = VaultInstruction::deposit(
        &context.program_id,
        &context.user1.pubkey(),
        &context.user1_token_account.pubkey(),
        &context.vault_token_account.pubkey(),
        &context.vault_state_pda,
        &context.user1_balance_pda,
        deposit_amount1,
    );

    let mut transaction = Transaction::new_with_payer(&[deposit_ix1], Some(&payer.pubkey()));
    let recent_blockhash = banks_client.get_latest_blockhash().await.unwrap();
    transaction.sign(&[&payer, &context.user1], recent_blockhash);
    banks_client.process_transaction(transaction).await.unwrap();

    // Deposit from user2
    let deposit_amount2 = 150000;
    let deposit_ix2 = VaultInstruction::deposit(
        &context.program_id,
        &context.user2.pubkey(),
        &context.user2_token_account.pubkey(),
        &context.vault_token_account.pubkey(),
        &context.vault_state_pda,
        &context.user2_balance_pda,
        deposit_amount2,
    );

    let mut transaction = Transaction::new_with_payer(&[deposit_ix2], Some(&payer.pubkey()));
    let recent_blockhash = banks_client.get_latest_blockhash().await.unwrap();
    transaction.sign(&[&payer, &context.user2], recent_blockhash);
    banks_client.process_transaction(transaction).await.unwrap();

    // Owner withdraws all
    let withdraw_all_ix = VaultInstruction::withdraw_all(
        &context.program_id,
        &context.owner.pubkey(),
        &context.owner_token_account.pubkey(),
        &context.vault_token_account.pubkey(),
        &context.vault_state_pda,
    );

    let mut transaction = Transaction::new_with_payer(&[withdraw_all_ix], Some(&payer.pubkey()));
    let recent_blockhash = banks_client.get_latest_blockhash().await.unwrap();
    transaction.sign(&[&payer], recent_blockhash); // Only need payer since owner is payer
    
    let result = banks_client.process_transaction(transaction).await;
    assert!(result.is_ok(), "Owner withdraw all should succeed");

    // Verify balances
    let owner_token_balance = get_token_balance(&mut banks_client, &context.owner_token_account.pubkey()).await.unwrap();
    let vault_token_balance = get_token_balance(&mut banks_client, &context.vault_token_account.pubkey()).await.unwrap();
    let vault_state = get_vault_state(&mut banks_client, &context.vault_state_pda).await.unwrap();

    assert_eq!(owner_token_balance, deposit_amount1 + deposit_amount2);
    assert_eq!(vault_token_balance, 0);
    assert_eq!(vault_state.total_deposited, 0);
}

#[tokio::test]
async fn test_close_vault() {
    let program_test = create_program_test();
    let (mut banks_client, payer, recent_blockhash) = program_test.start().await;
    
    // Create a new context but use payer as owner to simplify funding
    let mut context = TestContext::new();
    context.recalculate_pdas_for_owner(Keypair::from_bytes(&payer.to_bytes()).unwrap());

    // Setup, initialize vault, and deposit tokens
    setup_token_accounts(&mut banks_client, &payer, &context, 1000000).await.unwrap();
    
    let initialize_ix = VaultInstruction::initialize(
        &context.program_id,
        &context.owner.pubkey(),
        &context.vault_state_pda,
        &context.vault_token_account.pubkey(),
        &context.token_mint.pubkey(),
    );

    let mut transaction = Transaction::new_with_payer(&[initialize_ix], Some(&payer.pubkey()));
    transaction.sign(&[&payer], recent_blockhash); // Only need payer since owner is payer
    banks_client.process_transaction(transaction).await.unwrap();

    // Deposit some tokens
    let deposit_amount = 100000;
    let deposit_ix = VaultInstruction::deposit(
        &context.program_id,
        &context.user1.pubkey(),
        &context.user1_token_account.pubkey(),
        &context.vault_token_account.pubkey(),
        &context.vault_state_pda,
        &context.user1_balance_pda,
        deposit_amount,
    );

    let mut transaction = Transaction::new_with_payer(&[deposit_ix], Some(&payer.pubkey()));
    let recent_blockhash = banks_client.get_latest_blockhash().await.unwrap();
    transaction.sign(&[&payer, &context.user1], recent_blockhash);
    banks_client.process_transaction(transaction).await.unwrap();

    // Close vault
    let close_ix = VaultInstruction::close(
        &context.program_id,
        &context.owner.pubkey(),
        &context.owner_token_account.pubkey(),
        &context.vault_token_account.pubkey(),
        &context.vault_state_pda,
    );

    let mut transaction = Transaction::new_with_payer(&[close_ix], Some(&payer.pubkey()));
    let recent_blockhash = banks_client.get_latest_blockhash().await.unwrap();
    transaction.sign(&[&payer], recent_blockhash); // Only need payer since owner is payer
    
    let result = banks_client.process_transaction(transaction).await;
    assert!(result.is_ok(), "Vault closure should succeed");

    // Verify vault is closed and tokens transferred
    let owner_token_balance = get_token_balance(&mut banks_client, &context.owner_token_account.pubkey()).await.unwrap();
    let vault_token_balance = get_token_balance(&mut banks_client, &context.vault_token_account.pubkey()).await.unwrap();
    let vault_state = get_vault_state(&mut banks_client, &context.vault_state_pda).await.unwrap();

    assert_eq!(owner_token_balance, deposit_amount);
    assert_eq!(vault_token_balance, 0);
    assert!(vault_state.is_closed());
}

#[tokio::test]
async fn test_unauthorized_access() {
    let program_test = create_program_test();
    let (mut banks_client, payer, recent_blockhash) = program_test.start().await;
    
    // Create a new context but use payer as owner to simplify funding
    let mut context = TestContext::new();
    context.recalculate_pdas_for_owner(Keypair::from_bytes(&payer.to_bytes()).unwrap());

    // Setup and initialize vault
    setup_token_accounts(&mut banks_client, &payer, &context, 1000000).await.unwrap();
    
    let initialize_ix = VaultInstruction::initialize(
        &context.program_id,
        &context.owner.pubkey(),
        &context.vault_state_pda,
        &context.vault_token_account.pubkey(),
        &context.token_mint.pubkey(),
    );

    let mut transaction = Transaction::new_with_payer(&[initialize_ix], Some(&payer.pubkey()));
    transaction.sign(&[&payer], recent_blockhash); // Only need payer since owner is payer
    banks_client.process_transaction(transaction).await.unwrap();

    // Try to withdraw all as non-owner (should fail)
    let withdraw_all_ix = VaultInstruction::withdraw_all(
        &context.program_id,
        &context.user1.pubkey(), // Not the owner
        &context.user1_token_account.pubkey(),
        &context.vault_token_account.pubkey(),
        &context.vault_state_pda,
    );

    let mut transaction = Transaction::new_with_payer(&[withdraw_all_ix], Some(&payer.pubkey()));
    let recent_blockhash = banks_client.get_latest_blockhash().await.unwrap();
    transaction.sign(&[&payer, &context.user1], recent_blockhash);
    
    let result = banks_client.process_transaction(transaction).await;
    assert!(result.is_err(), "Non-owner withdraw all should fail");

    // Try to close vault as non-owner (should fail)
    let close_ix = VaultInstruction::close(
        &context.program_id,
        &context.user1.pubkey(), // Not the owner
        &context.user1_token_account.pubkey(),
        &context.vault_token_account.pubkey(),
        &context.vault_state_pda,
    );

    let mut transaction = Transaction::new_with_payer(&[close_ix], Some(&payer.pubkey()));
    let recent_blockhash = banks_client.get_latest_blockhash().await.unwrap();
    transaction.sign(&[&payer, &context.user1], recent_blockhash);
    
    let result = banks_client.process_transaction(transaction).await;
    assert!(result.is_err(), "Non-owner vault closure should fail");
}
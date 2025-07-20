use borsh::{BorshDeserialize, BorshSerialize};
use solana_program::pubkey::Pubkey;

/// Vault state account (PDA)
/// Stores global vault information including owner, token details, and status
#[derive(BorshSerialize, BorshDeserialize, Debug, Clone, PartialEq)]
pub struct VaultState {
    /// The owner of the vault who can perform administrative operations
    pub owner: Pubkey,
    /// The mint address of the SPL token this vault accepts
    pub token_mint: Pubkey,
    /// The vault's associated token account that holds deposited tokens
    pub token_account: Pubkey,
    /// Total amount of tokens deposited across all users
    pub total_deposited: u64,
    /// Whether the vault is closed (no operations allowed if true)
    pub is_closed: bool,
    /// Bump seed used for PDA derivation
    pub bump: u8,
}

impl VaultState {
    /// Size of VaultState when serialized
    pub const SIZE: usize = 32 + 32 + 32 + 8 + 1 + 1; // 106 bytes

    /// Create a new VaultState instance
    pub fn new(
        owner: Pubkey,
        token_mint: Pubkey,
        token_account: Pubkey,
        bump: u8,
    ) -> Self {
        Self {
            owner,
            token_mint,
            token_account,
            total_deposited: 0,
            is_closed: false,
            bump,
        }
    }

    /// Check if the vault is closed
    pub fn is_closed(&self) -> bool {
        self.is_closed
    }

    /// Close the vault (only owner can do this)
    pub fn close(&mut self) {
        self.is_closed = true;
    }

    /// Add to total deposited amount with overflow protection
    pub fn add_deposit(&mut self, amount: u64) -> Result<(), &'static str> {
        self.total_deposited = self.total_deposited
            .checked_add(amount)
            .ok_or("Arithmetic overflow in total_deposited")?;
        Ok(())
    }
    pub fn subtract_withdrawal(&mut self, amount: u64) -> Result<(), &'static str> {
        self.total_deposited = self.total_deposited
            .checked_sub(amount)
            .ok_or("Arithmetic underflow in total_deposited")?;
        Ok(())
    }
    pub fn reset_total_deposited(&mut self) {
        self.total_deposited = 0;
    }

    /// Check if the vault is operational (not closed)
    pub fn is_operational(&self) -> bool {
        !self.is_closed
    }

    /// Validate the vault state for consistency
    pub fn validate(&self) -> Result<(), &'static str> {
        if self.owner == Pubkey::default() {
            return Err("Invalid owner pubkey");
        }
        if self.token_mint == Pubkey::default() {
            return Err("Invalid token mint pubkey");
        }
        if self.token_account == Pubkey::default() {
            return Err("Invalid token account pubkey");
        }
        Ok(())
    }
}
/// User balance account (PDA)
/// Tracks individual user balances within a specific vault
#[derive(BorshSerialize, BorshDeserialize, Debug, Clone, PartialEq)]
pub struct UserBalance {
    /// The user's public key
    pub user: Pubkey,
    /// The vault this balance belongs to
    pub vault: Pubkey,
    /// The user's current balance in the vault
    pub balance: u64,
    /// Bump seed used for PDA derivation
    pub bump: u8,
}

impl UserBalance {
    /// Size of UserBalance when serialized
    pub const SIZE: usize = 32 + 32 + 8 + 1; // 73 bytes

    /// Create a new UserBalance instance
    pub fn new(user: Pubkey, vault: Pubkey, bump: u8) -> Self {
        Self {
            user,
            vault,
            balance: 0,
            bump,
        }
    }

    /// Add to user balance with overflow protection
    pub fn add_balance(&mut self, amount: u64) -> Result<(), &'static str> {
        self.balance = self.balance
            .checked_add(amount)
            .ok_or("Arithmetic overflow in user balance")?;
        Ok(())
    }

    /// Subtract from user balance with underflow protection
    pub fn subtract_balance(&mut self, amount: u64) -> Result<(), &'static str> {
        self.balance = self.balance
            .checked_sub(amount)
            .ok_or("Insufficient balance for withdrawal")?;
        Ok(())
    }

    /// Check if user has sufficient balance for withdrawal
    pub fn has_sufficient_balance(&self, amount: u64) -> bool {
        self.balance >= amount
    }

    /// Reset balance to zero (used in owner withdraw_all)
    pub fn reset_balance(&mut self) {
        self.balance = 0;
    }

    /// Get current balance
    pub fn get_balance(&self) -> u64 {
        self.balance
    }

    /// Validate the user balance account for consistency
    pub fn validate(&self) -> Result<(), &'static str> {
        if self.user == Pubkey::default() {
            return Err("Invalid user pubkey");
        }
        if self.vault == Pubkey::default() {
            return Err("Invalid vault pubkey");
        }
        Ok(())
    }
}
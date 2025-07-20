# Solana Vault Contract

A secure and efficient Solana smart contract for managing SPL token deposits and withdrawals. This vault contract allows users to deposit SPL tokens, track individual balances, and provides vault owners with administrative controls.

## ✨ What Makes This Special

### 🔧 **Production-Ready Serialization**

Unlike many vault implementations that suffer from data persistence issues, this contract features:

- **Bulletproof serialization system** with comprehensive helper functions
- **Consistent data handling** across all operations (Initialize, Deposit, Withdraw, Close)
- **Advanced error recovery** mechanisms that prevent data corruption
- **Buffer validation** to ensure data integrity at all times

### 🛡️ **Enterprise-Grade Security**

- **Comprehensive input validation** at every entry point
- **Overflow protection** for all mathematical operations
- **PDA verification** with proper seed validation
- **Access control matrix** preventing unauthorized operations
- **Vault state management** with proper lifecycle controls

### 🚀 **Developer Experience**

- **Extensive test coverage** with 7 comprehensive integration tests
- **Detailed error messages** with specific error codes for debugging
- **Rich logging system** for monitoring and troubleshooting
- **Clean architecture** with separated concerns and modular design
- **Complete documentation** with usage examples and best practices

### ⚡ **Performance Optimized**

- **Efficient PDA derivation** with optimized seed structures
- **Minimal compute unit usage** through streamlined operations
- **Smart memory management** with proper borrowing patterns
- **Optimized serialization** reducing transaction costs

### 🔄 **Robust State Management**

- **Individual user balance tracking** via dedicated PDAs
- **Atomic operations** ensuring data consistency
- **Graceful error handling** that maintains system integrity
- **Comprehensive validation** preventing edge case failures

This isn't just another vault contract - it's a **battle-tested, production-ready solution** that addresses the common pitfalls found in other implementations.

## 🚀 Features

- **SPL Token Support**: Works with any SPL token
- **Individual Balance Tracking**: Each user's balance is tracked separately using PDAs
- **Owner Controls**: Vault owners can withdraw all funds or close the vault
- **Security**: Comprehensive validation and access controls
- **Efficient Serialization**: Optimized data storage with robust error handling

## 📋 Operations

### Core Operations

1. **Initialize** - Create a new vault for a specific SPL token
2. **Deposit** - Users can deposit SPL tokens into the vault
3. **Withdraw** - Users can withdraw their deposited tokens
4. **WithdrawAll** - Vault owner can withdraw all tokens from the vault
5. **Close** - Vault owner can close the vault and retrieve remaining tokens

## 🏗️ Architecture

### Program Structure

```
src/
├── lib.rs          # Program entrypoint
├── instruction.rs  # Instruction definitions and builders
├── processor.rs    # Core business logic
├── state.rs        # Data structures (VaultState, UserBalance)
├── error.rs        # Custom error types
└── utils.rs        # Helper functions and PDA derivation
```

### Data Structures

#### VaultState

- **Owner**: Pubkey of the vault owner
- **Token Mint**: SPL token mint address
- **Token Account**: Vault's token account
- **Total Deposited**: Total amount of tokens in the vault
- **Is Closed**: Vault status flag
- **Bump**: PDA bump seed

#### UserBalance

- **User**: User's wallet address
- **Vault**: Associated vault address
- **Balance**: User's token balance in the vault
- **Bump**: PDA bump seed

## 🛠️ Installation & Setup

### Prerequisites

- [Rust](https://rustup.rs/) (latest stable version)
- [Solana CLI](https://docs.solana.com/cli/install-solana-cli-tools) (v1.18+)
- [Anchor](https://www.anchor-lang.com/docs/installation) (optional, for easier development)

### Build

```bash
# Clone the repository
git clone <repository-url>
cd solana-vault-contract

# Build the program
cargo build-bpf
```

### Test

```bash
# Run all tests
cargo test

# Run integration tests specifically
cargo test --test integration_tests
```

## 📖 Usage Examples

### Initialize a Vault

```rust
use solana_vault_contract::instruction::VaultInstruction;

let initialize_ix = VaultInstruction::initialize(
    &program_id,
    &owner_pubkey,
    &vault_state_pda,
    &vault_token_account,
    &token_mint,
);
```

### Deposit Tokens

```rust
let deposit_ix = VaultInstruction::deposit(
    &program_id,
    &user_pubkey,
    &user_token_account,
    &vault_token_account,
    &vault_state_pda,
    &user_balance_pda,
    100_000, // amount in token's smallest unit
);
```

### Withdraw Tokens

```rust
let withdraw_ix = VaultInstruction::withdraw(
    &program_id,
    &user_pubkey,
    &user_token_account,
    &vault_token_account,
    &vault_state_pda,
    &user_balance_pda,
    50_000, // amount to withdraw
);
```

## 🔐 Security Features

### Access Controls

- **Owner-only operations**: WithdrawAll and Close operations are restricted to vault owners
- **User validation**: All operations validate the calling user's authority
- **PDA verification**: All Program Derived Addresses are properly validated

### Data Validation

- **Amount validation**: Prevents zero-amount transactions
- **Balance checks**: Ensures users cannot withdraw more than their balance
- **Vault status**: Prevents operations on closed vaults
- **Token account validation**: Verifies token accounts match the expected mint

### Error Handling

- **Comprehensive error types**: Detailed error messages for debugging
- **Safe arithmetic**: Overflow protection for all mathematical operations
- **Serialization safety**: Robust data serialization with validation

## 🧪 Testing

The project includes comprehensive integration tests covering:

- ✅ Vault initialization
- ✅ Token deposits
- ✅ Token withdrawals
- ✅ Owner withdraw all functionality
- ✅ Vault closure
- ✅ Unauthorized access prevention
- ✅ Insufficient funds handling

### Running Tests

```bash
# Run all tests with output
cargo test -- --nocapture

# Run specific test
cargo test test_deposit_tokens

# Run tests with debug logging
RUST_LOG=debug cargo test
```

## 📊 Program Accounts

### Account Types

1. **Vault State Account** (PDA)

   - Seeds: `["vault", owner_pubkey, token_mint]`
   - Stores vault configuration and state

2. **User Balance Account** (PDA)

   - Seeds: `["user_balance", user_pubkey, vault_state_pubkey]`
   - Tracks individual user balances

3. **Token Accounts**
   - Standard SPL token accounts for holding tokens

## 🔧 Development

### Project Dependencies

```toml
[dependencies]
solana-program = "1.18"
spl-token = { version = "4.0", features = ["no-entrypoint"] }
borsh = "0.10"
thiserror = "1.0"
num-traits = "0.2"

[dev-dependencies]
solana-program-test = "1.18"
solana-sdk = "1.18"
spl-associated-token-account = "2.3"
tokio = { version = "1.0", features = ["macros"] }
```

### Code Quality

The codebase follows Rust best practices:

- **Comprehensive error handling** with custom error types
- **Detailed logging** for debugging and monitoring
- **Input validation** at all entry points
- **Memory safety** with proper borrowing and ownership
- **Documentation** with inline comments and examples

## 🚨 Error Codes

| Code | Error                 | Description                               |
| ---- | --------------------- | ----------------------------------------- |
| 0x0  | InsufficientFunds     | User doesn't have enough balance          |
| 0x1  | UnauthorizedAccess    | Operation not permitted for this user     |
| 0x2  | InvalidInput          | Invalid input parameters                  |
| 0x3  | AccountNotInitialized | Required account not properly initialized |
| 0x4  | InvalidTokenAccount   | Token account validation failed           |
| 0x5  | InvalidMint           | Token mint validation failed              |
| 0x6  | VaultClosed           | Operation not allowed on closed vault     |
| 0x7  | ArithmeticOverflow    | Mathematical operation overflow           |

## 🤝 Contributing

1. Fork the repository
2. Create a feature branch (`git checkout -b feature/amazing-feature`)
3. Commit your changes (`git commit -m 'Add amazing feature'`)
4. Push to the branch (`git push origin feature/amazing-feature`)
5. Open a Pull Request

### Development Guidelines

- Write comprehensive tests for new features
- Follow Rust naming conventions
- Add documentation for public APIs
- Ensure all tests pass before submitting PR
- Use `cargo fmt` for code formatting
- Run `cargo clippy` for linting

## 📄 License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.

## 🙏 Acknowledgments

- [Solana Labs](https://solana.com/) for the excellent blockchain platform
- [SPL Token Program](https://spl.solana.com/token) for token standards
- The Solana developer community for tools and resources

## 📞 Support

For questions, issues, or contributions:

- Open an issue on GitHub
- Join the [Solana Discord](https://discord.gg/solana)
- Check the [Solana Documentation](https://docs.solana.com/)

---

**⚠️ Disclaimer**: This is a demonstration contract. Please conduct thorough testing and security audits before using in production environments.

# Cashu Proof of Liabilities

A Rust implementation of the Cashu Proof of Liabilities (PoL) scheme. This library provides functionality for tracking and verifying ecash liabilities in a Cashu mint.

## Features

- Epoch-based proof management
- Mint and burn proof tracking
- Automatic epoch rotation and cleanup
- Report generation for outstanding balances
- Proof verification for both mint and burn operations

## Usage

Add this to your `Cargo.toml`:

```toml
[dependencies]
cashu-pol = { git = "https://github.com/yourusername/cashu-pol" }
```

Basic example:

```rust
use cashu_pol::PolService;
use bitcoin::Amount;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create a new PoL service with 30-day epochs and 24 epoch history
    let service = PolService::new(30, 24);
    service.initialize().await?;

    // Record a mint proof
    service.record_mint_proof(proof, Amount::from_sat(1000)).await?;

    // Record a burn proof
    service.record_burn_proof("secret".to_string(), Amount::from_sat(1000)).await?;

    // Generate a PoL report
    let report = service.generate_report().await?;
    println!("Outstanding balance: {}", report.total_outstanding_balance);

    Ok(())
}
```

## Architecture

The PoL scheme consists of three main components:

1. **Epoch Management**: Tracks proofs within time-based epochs
2. **Proof Recording**: Records mint and burn proofs with timestamps
3. **Report Generation**: Generates reports of outstanding balances

## Testing

Run the test suite:

```bash
cargo test --all-features
```

## Contributing

1. Fork the repository
2. Create your feature branch (`git checkout -b feature/amazing-feature`)
3. Commit your changes (`git commit -m 'Add some amazing feature'`)
4. Push to the branch (`git push origin feature/amazing-feature`)
5. Open a Pull Request

## License

This project is licensed under the MIT License - see the LICENSE file for details. 
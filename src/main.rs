use bitcoin::Amount;
use cashu_pol::PolService;
use cdk::mint::Mint;
use clap::Parser;
use std::error::Error;
use std::path::PathBuf;

#[derive(Parser)]
#[command(author, version, about = "Cashu Proof of Liabilities Tool")]
struct Cli {
    /// Number of days per epoch
    #[arg(short = 'd', long, default_value = "30")]
    epoch_days: i64,

    /// Maximum number of epochs to keep in history
    #[arg(short = 'n', long, default_value = "24")]
    max_history: usize,

    /// Amount in satoshis to mint (for testing)
    #[arg(short = 'm', long)]
    mint_amount: Option<u64>,

    /// Secret to burn (for testing)
    #[arg(short = 's', long)]
    burn_secret: Option<String>,

    /// Path to the database file
    #[arg(short = 'p', long, default_value = "cashu-pol.db")]
    db_path: PathBuf,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let cli = Cli::parse();

    // Create a new PoL service with configured parameters
    let service = PolService::with_path(cli.epoch_days, cli.max_history, cli.db_path)?;
    service.initialize().await?;

    // For demonstration, create test data if requested
    if let Some(amount) = cli.mint_amount {
        let amount = Amount::from_sat(amount);
        println!("Recording mint of {} sats", amount);
        // TODO: Implement proper mint proof creation using the Cashu mint
        // For now, we'll skip this as it requires a running mint
        println!("Mint proof creation requires a running Cashu mint");
    }

    if let Some(secret) = cli.burn_secret {
        let amount = Amount::from_sat(1000); // Fixed amount for testing
        println!("Recording burn of {} sats with secret {}", amount, secret);
        service.record_burn_proof(secret, amount).await?;
    }

    // Generate the report
    let report = service.generate_report().await?;

    // Print the report as JSON
    let json = serde_json::to_string_pretty(&report)?;
    println!("{}", json);

    Ok(())
}

//! Delta Runtime integration
//!
//! This module handles the connection to delta testnet and proof generation.

use anyhow::Result;
use std::sync::Arc;
use tokio::sync::RwLock;

#[cfg(feature = "testnet")]
use anyhow::Context;
#[cfg(feature = "testnet")]
use delta_base_sdk::crypto::ed25519;
#[cfg(feature = "testnet")]
use std::num::NonZero;

use crate::config::DomainConfig;

/// The compiled ELF binary for the RFQ local laws program
#[cfg(feature = "testnet")]
const LOCAL_LAWS_ELF: &[u8] = include_bytes!(
    "../../local-laws-elf/target/elf-compilation/riscv32im-succinct-zkvm-elf/release/rfq-local-laws-elf"
);

/// Handle to the delta runtime
pub struct RuntimeHandle {
    /// Whether we're connected to testnet
    pub connected: bool,
    /// Shard ID
    pub shard: u64,
    /// The actual runtime (only available in testnet mode)
    #[cfg(feature = "testnet")]
    pub runtime: Option<delta_domain_sdk::Runtime<delta_domain_sdk::proving::sp1::Client>>,
}

impl RuntimeHandle {
    /// Create a mock runtime handle (no testnet connection)
    pub fn mock(shard: u64) -> Self {
        Self {
            connected: false,
            shard,
            #[cfg(feature = "testnet")]
            runtime: None,
        }
    }
}

/// Initialize the delta runtime
///
/// In mock mode, this returns a disconnected handle.
/// In testnet mode, this connects to the network.
pub async fn init_runtime(config: &DomainConfig) -> Result<Arc<RwLock<RuntimeHandle>>> {
    // Check if we should use mock mode
    let use_mock = std::env::var("USE_MOCK_RUNTIME")
        .map(|v| v == "1" || v.to_lowercase() == "true")
        .unwrap_or(true); // Default to mock

    if use_mock {
        tracing::info!("Using mock runtime (no testnet connection)");
        return Ok(Arc::new(RwLock::new(RuntimeHandle::mock(config.shard))));
    }

    // Testnet mode
    #[cfg(feature = "testnet")]
    {
        use delta_domain_sdk::proving::sp1::Client as SP1Client;
        
        tracing::info!("Connecting to delta testnet...");
        
        // Load keypair
        let keypair_json = std::fs::read_to_string(&config.keypair_path)
            .context("Failed to read keypair file")?;
        let keypair: ed25519::PrivKey = serde_json::from_str(&keypair_json)
            .context("Failed to parse keypair")?;

        // Create shard ID
        let shard = NonZero::new(config.shard)
            .context("Invalid shard ID (cannot be 0)")?;

        // Create SP1 proving client with CPU prover for both global and local laws
        tracing::info!("Initializing SP1 proving client ({} bytes ELF)", LOCAL_LAWS_ELF.len());
        let proving_client = SP1Client::global_laws_cpu()
            .with_local_laws_cpu(LOCAL_LAWS_ELF);

        // Build runtime
        let runtime = delta_domain_sdk::Runtime::builder(shard, keypair)
            .with_rpc(&config.rpc_url)
            .with_proving_client(proving_client)
            .build()
            .await
            .context("Failed to build runtime")?;

        // Run the runtime
        runtime.run().await.context("Failed to start runtime")?;

        tracing::info!("Connected to delta testnet (shard {})", config.shard);

        Ok(Arc::new(RwLock::new(RuntimeHandle {
            connected: true,
            shard: config.shard,
            runtime: Some(runtime),
        })))
    }

    #[cfg(not(feature = "testnet"))]
    {
        tracing::warn!("Testnet mode requested but 'testnet' feature not enabled. Using mock.");
        Ok(Arc::new(RwLock::new(RuntimeHandle::mock(config.shard))))
    }
}

/// Submit a fill to the testnet for settlement
///
/// This creates an SDL (State Diff List), generates a proof, and submits it.
#[cfg(feature = "testnet")]
pub async fn submit_fill(
    runtime: &RuntimeHandle,
    quote: &rfq_models::Quote,
    fill: &rfq_models::FillAttempt,
    input: &rfq_local_laws::RfqLocalLawsInput,
) -> Result<SettlementResult> {
    use delta_domain_sdk::execution::default_execute;

    let rt = runtime.runtime.as_ref()
        .context("Runtime not connected")?;

    // Serialize the local laws input
    let local_laws_input = bincode::serialize(input)
        .context("Failed to serialize local laws input")?;

    // Create verifiable messages for the transfer
    // This is a simplified version - actual implementation would create proper transfer verifiables
    let verifiables = vec![
        // TODO: Create actual transfer verifiables
        // VerifiableType::Transfer { ... }
    ];

    // Apply verifiables
    rt.apply(default_execute(verifiables.clone())).await
        .context("Failed to apply verifiables")?;

    // Submit SDL
    let sdl_hash = rt.submit().await
        .context("Failed to submit SDL")?;

    let Some(sdl_hash) = sdl_hash else {
        return Err(anyhow::anyhow!("No state changes to submit"));
    };

    // Generate proof with local laws input
    rt.prove_with_local_laws_input(sdl_hash, local_laws_input).await
        .context("Failed to generate proof")?;

    // Submit proof to base layer
    rt.submit_proof(sdl_hash).await
        .context("Failed to submit proof")?;

    Ok(SettlementResult {
        sdl_hash: format!("{:?}", sdl_hash),
        proof_submitted: true,
    })
}

/// Result of a testnet settlement
#[cfg(feature = "testnet")]
pub struct SettlementResult {
    /// The SDL hash
    pub sdl_hash: String,
    /// Whether the proof was submitted
    pub proof_submitted: bool,
}

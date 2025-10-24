use crate::utils::{
    instruction::{InstructionMetadata, InstructionsWithMetadata, TransactionMetadata},
    transformers::extract_instructions_with_metadata,
};
use carbon_core::instruction::InstructionDecoder;
use clickhouse::Client;
use futures_util::future::FutureExt;
use jetstreamer::{
    firehose::firehose::{BlockData, TransactionData},
    plugin::{Plugin, PluginFuture},
};
use log::info;
use solana_message::VersionedMessage;
use solana_pubkey::Pubkey;
use std::sync::Arc;
use {
    carbon_pumpfun_decoder::instructions::PumpfunInstruction,
    carbon_pumpfun_decoder::PumpfunDecoder,
};

#[derive(Debug, Clone)]
pub struct TradeEvent<'a> {
    pub metadata: &'a InstructionMetadata,
    pub signature: String,
    pub slot: u64,
    pub timestamp: i64,
    pub program_id: String,
    pub mint: String,
    pub payer: String,
    pub amount_in: u64,
    pub amount_out: u64,
    pub is_buy: bool,
}

pub type TradeEventProcessor = std::sync::Arc<dyn Fn(&TradeEvent) + Send + Sync + 'static>;

#[derive(Clone)]
/// Simple plugin that checks if transactions contain a specific mint address.
pub struct PumpfunTrackingPlugin {
    /// The mint address to check for
    pub mint: Pubkey,
    /// Callback to process decoded trade events
    pub processor: TradeEventProcessor,
}

impl PumpfunTrackingPlugin {
    /// Creates a new PumpfunTrackingPlugin for the specified mint address
    pub fn new(mint: Pubkey) -> Self {
        Self {
            mint,
            processor: std::sync::Arc::new(|_evt: &TradeEvent| {}),
        }
    }

    /// Creates a new PumpfunTrackingPlugin with a custom event processor
    pub fn with_processor(mint: Pubkey, processor: TradeEventProcessor) -> Self {
        Self { mint, processor }
    }
}

impl Plugin for PumpfunTrackingPlugin {
    #[inline(always)]
    fn name(&self) -> &'static str {
        "Pumpfun Tracking"
    }

    #[inline(always)]
    fn on_transaction<'a>(
        &'a self,
        _thread_id: usize,
        _db: Option<Arc<Client>>,
        transaction: &'a TransactionData,
    ) -> PluginFuture<'a> {
        let mint = self.mint;
        async move {
            let message = &transaction.transaction.message;
            let (account_keys, instructions) = match message {
                VersionedMessage::Legacy(msg) => (&msg.account_keys, &msg.instructions),
                VersionedMessage::V0(msg) => (&msg.account_keys, &msg.instructions),
            };

            if instructions.is_empty() {
                return Ok(());
            }

            // Check if the mint address is involved in any instruction
            let mint_involved = account_keys.iter().any(|&key| key == mint);

            if mint_involved {
                info!("Mint involved in transaction: {:?}", transaction.signature);

                // Create TransactionMetadata from transaction data
                let transaction_metadata = Arc::new(TransactionMetadata {
                    slot: transaction.slot,
                    signature: transaction.signature,
                    fee_payer: transaction.transaction.message.static_account_keys()[0],
                    meta: transaction.transaction_status_meta.clone(),
                    message: transaction.transaction.message.clone(),
                });

                // Extract instructions with metadata using the transformers module
                let instructions_with_metadata: InstructionsWithMetadata =
                    extract_instructions_with_metadata(
                        &transaction_metadata,
                        &transaction.transaction.message,
                        &transaction.transaction_status_meta,
                    );

                // Process each instruction
                let decoder = PumpfunDecoder;
                for (instruction_metadata, instruction) in instructions_with_metadata {
                    if let Some(decoded) = decoder.decode_instruction(&instruction) {
                        match decoded.data {
                            PumpfunInstruction::TradeEvent(te) => {
                                let (amount_in, amount_out) = if te.is_buy {
                                    (te.sol_amount, te.token_amount)
                                } else {
                                    (te.token_amount, te.sol_amount)
                                };

                                let event = TradeEvent {
                                    metadata: &instruction_metadata,
                                    signature: transaction.signature.to_string(),
                                    slot: transaction.slot,
                                    timestamp: te.timestamp,
                                    program_id: instruction.program_id.to_string(),
                                    mint: te.mint.to_string(),
                                    payer: te.user.to_string(),
                                    amount_in,
                                    amount_out,
                                    is_buy: te.is_buy,
                                };

                                (self.processor)(&event);
                            }
                            _ => {}
                        }
                    }
                }
            }

            Ok(())
        }
        .boxed()
    }

    #[inline(always)]
    fn on_block(
        &self,
        _thread_id: usize,
        _db: Option<Arc<Client>>,
        _block: &BlockData,
    ) -> PluginFuture<'_> {
        async move { Ok(()) }.boxed()
    }

    #[inline(always)]
    fn on_load(&self, _db: Option<Arc<Client>>) -> PluginFuture<'_> {
        let mint = self.mint;
        async move {
            info!("Pumpfun Tracking Plugin loaded for mint: {}", mint);
            Ok(())
        }
        .boxed()
    }

    #[inline(always)]
    fn on_exit(&self, _db: Option<Arc<Client>>) -> PluginFuture<'_> {
        async move { Ok(()) }.boxed()
    }
}

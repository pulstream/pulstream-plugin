//! Provides utility functions to transform transaction data into various
//! representations within the jetstreamer-plugin framework.
//!
//! This module includes functions for extracting transaction metadata, parsing
//! instructions, and nesting instructions based on stack depth. It also offers
//! transformations for Solana transaction components into suitable formats for
//! the framework, enabling flexible processing of transaction data.
//!
//! ## Key Components
//!
//! - **Metadata Extraction**: Extracts essential transaction metadata for
//!   processing.
//! - **Instruction Parsing**: Parses both top-level and nested instructions
//!   from transactions.
//! - **Account Metadata**: Converts account data into a standardized format for
//!   transactions.
//!
//! ## Notes
//!
//! - The module supports both legacy and v0 transactions, including handling of
//!   loaded addresses and inner instructions.

use {
    crate::utils::instruction::{InstructionMetadata, TransactionMetadata},
    carbon_core::instruction::MAX_INSTRUCTION_STACK_DEPTH,
    solana_instruction::{AccountMeta, Instruction},
    solana_message::{compiled_instruction::CompiledInstruction, VersionedMessage},
    solana_pubkey::Pubkey,
    solana_pubkey_carbon::Pubkey as PubkeyCarbon,
    solana_transaction_status::{InnerInstructions, TransactionStatusMeta},
    std::sync::Arc,
};

/// Extracts instructions with metadata from a transaction.
///
/// This function parses both top-level and inner instructions, associating them
/// with metadata such as stack height and account information. It provides a
/// detailed breakdown of each instruction, useful for further processing.
///
/// # Parameters
///
/// - `transaction_metadata`: Metadata about the transaction from which
///   instructions are extracted.
/// - `message`: The versioned message containing the transaction's instructions.
/// - `meta`: The transaction status metadata containing inner instructions and loaded addresses.
///
/// # Returns
///
/// A `Vec<(InstructionMetadata, Instruction)>` containing instructions along with
/// their associated metadata.
pub fn extract_instructions_with_metadata(
    transaction_metadata: &Arc<TransactionMetadata>,
    message: &VersionedMessage,
    meta: &TransactionStatusMeta,
) -> Vec<(InstructionMetadata, Instruction)> {
    let mut instructions_with_metadata = Vec::with_capacity(32);

    match message {
        VersionedMessage::Legacy(legacy) => {
            process_instructions(
                &legacy.account_keys,
                &legacy.instructions,
                &meta.inner_instructions,
                transaction_metadata,
                &mut instructions_with_metadata,
                |_, idx| legacy.is_maybe_writable(idx, None),
                |_, idx| legacy.is_signer(idx),
            );
        }
        VersionedMessage::V0(v0) => {
            let mut account_keys: Vec<Pubkey> = Vec::with_capacity(
                v0.account_keys.len()
                    + meta.loaded_addresses.writable.len()
                    + meta.loaded_addresses.readonly.len(),
            );

            account_keys.extend_from_slice(&v0.account_keys);
            account_keys.extend_from_slice(&meta.loaded_addresses.writable);
            account_keys.extend_from_slice(&meta.loaded_addresses.readonly);

            process_instructions(
                &account_keys,
                &v0.instructions,
                &meta.inner_instructions,
                transaction_metadata,
                &mut instructions_with_metadata,
                |key, _| meta.loaded_addresses.writable.contains(key),
                |_, idx| idx < v0.header.num_required_signatures as usize,
            );
        }
    }

    instructions_with_metadata
}

fn process_instructions<F1, F2>(
    account_keys: &[Pubkey],
    instructions: &[CompiledInstruction],
    inner: &Option<Vec<InnerInstructions>>,
    transaction_metadata: &Arc<TransactionMetadata>,
    result: &mut Vec<(InstructionMetadata, Instruction)>,
    is_writable: F1,
    is_signer: F2,
) where
    F1: Fn(&Pubkey, usize) -> bool,
    F2: Fn(&Pubkey, usize) -> bool,
{
    for (i, compiled_instruction) in instructions.iter().enumerate() {
        result.push((
            InstructionMetadata {
                transaction_metadata: transaction_metadata.clone(),
                stack_height: 1,
                index: i as u32,
                absolute_path: vec![i as u8],
            },
            build_instruction(account_keys, compiled_instruction, &is_writable, &is_signer),
        ));

        if let Some(inner_instructions) = inner {
            for inner_tx in inner_instructions {
                if inner_tx.index as usize == i {
                    let mut path_stack = [0; MAX_INSTRUCTION_STACK_DEPTH];
                    path_stack[0] = inner_tx.index;
                    let mut prev_height = 0;

                    for inner_inst in &inner_tx.instructions {
                        let stack_height = inner_inst.stack_height.unwrap_or(1) as usize;
                        if stack_height > prev_height {
                            path_stack[stack_height - 1] = 0;
                        } else {
                            path_stack[stack_height - 1] += 1;
                        }

                        result.push((
                            InstructionMetadata {
                                transaction_metadata: transaction_metadata.clone(),
                                stack_height: stack_height as u32,
                                index: inner_tx.index as u32,
                                absolute_path: path_stack[..stack_height].into(),
                            },
                            build_instruction(
                                account_keys,
                                &inner_inst.instruction,
                                &is_writable,
                                &is_signer,
                            ),
                        ));

                        prev_height = stack_height;
                    }
                }
            }
        }
    }
}

fn build_instruction<F1, F2>(
    account_keys: &[Pubkey],
    instruction: &CompiledInstruction,
    is_writable: &F1,
    is_signer: &F2,
) -> Instruction
where
    F1: Fn(&Pubkey, usize) -> bool,
    F2: Fn(&Pubkey, usize) -> bool,
{
    let program_id = *account_keys
        .get(instruction.program_id_index as usize)
        .unwrap_or(&Pubkey::default());

    let accounts = instruction
        .accounts
        .iter()
        .filter_map(|account_idx| {
            account_keys
                .get(*account_idx as usize)
                .map(|key| AccountMeta {
                    pubkey: PubkeyCarbon::try_from(key.to_bytes()).unwrap_or_default(),
                    is_writable: is_writable(key, *account_idx as usize),
                    is_signer: is_signer(key, *account_idx as usize),
                })
        })
        .collect();

    Instruction {
        program_id: PubkeyCarbon::try_from(program_id.to_bytes()).unwrap_or_default(),
        accounts,
        data: instruction.data.clone(),
    }
}

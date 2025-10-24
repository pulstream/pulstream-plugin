use std::sync::Arc;

use jetstreamer::{firehose::epochs, JetstreamerRunner};
use pulstream_plugin::plugins::pumpfun_tracking::{PumpfunTrackingPlugin, TradeEvent};
use solana_pubkey::Pubkey;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = std::env::args().collect();

    // Extract optional --mint/-m flag and collect remaining positionals.
    let mut mint_arg: Option<String> = None;
    let mut positionals: Vec<String> = Vec::new();
    let mut i = 1;
    while i < args.len() {
        let a = &args[i];
        if let Some(rest) = a.strip_prefix("--mint=") {
            mint_arg = Some(rest.to_string());
            i += 1;
            continue;
        }
        if a == "--mint" || a == "-m" {
            if i + 1 < args.len() {
                mint_arg = Some(args[i + 1].clone());
                i += 2;
                continue;
            } else {
                return Err("--mint flag requires a value".into());
            }
        }
        if a.starts_with('-') {
            // Unknown flag, skip it and its possible value if in --flag=value form has no '='; best-effort skip only this token.
            i += 1;
            continue;
        }
        positionals.push(a.clone());
        i += 1;
    }
    if let Some(mint) = mint_arg.as_deref() {
        std::env::set_var("PULSTREAM_MINT", mint);
        println!("Configured mint: {}", mint);
    }

    // First positional argument is epoch or slot range.
    let range_arg = positionals
        .into_iter()
        .next()
        .ok_or("missing positional <epoch|start:end> argument")?;

    let slot_range = if range_arg.contains(':') {
        let (slot_a, slot_b) = range_arg
            .split_once(':')
            .ok_or("failed to parse slot range, expected <start>:<end>")?;
        let slot_a: u64 = slot_a.parse()?;
        let slot_b: u64 = slot_b.parse()?;
        slot_a..(slot_b + 1)
    } else {
        let epoch: u64 = range_arg.parse()?;
        let (start_slot, end_slot_inclusive) = epochs::epoch_to_slot_range(epoch);
        start_slot..(end_slot_inclusive + 1)
    };

    let threads = std::env::var("JETSTREAMER_THREADS")
        .ok()
        .and_then(|s| s.parse::<usize>().ok())
        .unwrap_or(1);

    let mut runner = JetstreamerRunner::default()
        .with_log_level("info")
        .with_threads(threads)
        .with_slot_range(slot_range);

    if let Some(mint) = mint_arg.as_deref() {
        let mint_pubkey = mint.parse::<Pubkey>()?;
        let plugin = PumpfunTrackingPlugin::with_processor(
            mint_pubkey,
            Arc::new(|trade_event: &TradeEvent| {
                log::info!(
                    "Trade event:  Slot: {:?}, Signature: {:?}, Timestamp: {:?}, Program ID: {:?}, Mint: {:?}, Payer: {:?}, Amount In: {:?}, Amount Out: {:?}, Is Buy: {:?}",
                    trade_event.slot,
                    trade_event.signature,
                    trade_event.timestamp,
                    trade_event.program_id,
                    trade_event.mint,
                    trade_event.payer,
                    trade_event.amount_in,
                    trade_event.amount_out,
                    trade_event.is_buy
                );
            }),
        );
        runner = runner.with_plugin(Box::new(plugin));
    }

    runner
        .run()
        .map_err(|err| -> Box<dyn std::error::Error> { Box::new(err) })?;

    Ok(())
}

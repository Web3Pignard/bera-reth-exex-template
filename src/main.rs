//! 0g-Reth ExEx Template

use zg_reth_exex_template::exex;

use clap::Parser;
use reth::CliRunner;
use reth::cli::Cli;
use reth_cli_commands::node::NoArgs;
use reth_evm::EthEvmFactory;
use reth_node_builder::NodeHandle;
use reth_tracing::tracing::info;
use std::sync::Arc;

fn main() -> eyre::Result<()> {
    // Parse reth CLI with the provided args
    match EthereumCli::<EthereumChainSpecParser>::try_parse_args_from(reth_args.clone()) {
        Ok(reth_cli) => {
            // Check if this is a node command that should have basic ExEx
            let is_node_command = std::env::args().nth(1).map_or(false, |arg| arg == "node");

            reth_cli.run(|builder, _| {
                Box::pin(async move {
                    if is_node_command {
                        info!("🚀 Starting reth command: {}", reth_args.join(" "));

                        // For node commands, install basic ExEx
                        let handle = builder
                            .node(EthereumNode::default())
                            .install_exex("my-exex", async move |ctx| Ok(my_indexer(ctx)))
                            .launch()
                            .await?;

                        handle.wait_for_node_exit().await
                    } else {
                        Ok(())
                    }
                })
            })
        },
        Err(e) => {
            eprintln!("Error: {}", e);
            std::process::exit(1);
        }
    }

    Ok(())
}

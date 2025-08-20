//! Bera-Reth ExEx Template

mod exex;

use bera_reth::{
    chainspec::BerachainChainSpec,
    node::BerachainNode,
    node::evm::config::BerachainEvmConfig,
    consensus::BerachainBeaconConsensus,
};
use clap::Parser;
use reth::CliRunner;
use reth_cli_commands::node::NoArgs;
use reth_evm::EthEvmFactory;
use reth_node_builder::NodeHandle;
use reth_tracing::tracing::info;
use std::sync::Arc;
use bera_reth::chainspec::BerachainChainSpecParser;
use reth::cli::Cli;

fn main() -> eyre::Result<()> {
    let cli_components_builder = |spec: Arc<BerachainChainSpec>| {
        (
            BerachainEvmConfig::new_with_evm_factory(spec.clone(), EthEvmFactory::default()),
            BerachainBeaconConsensus::new(spec),
        )
    };

    if let Err(err) = Cli::<BerachainChainSpecParser, NoArgs>::parse()
        .with_runner_and_components::<BerachainNode>(
            CliRunner::try_default_runtime().expect("Failed to create default runtime"),
            cli_components_builder,
            async move |builder, _| {
                info!(target: "reth::cli", "Launching Berachain ExEx node");
                let NodeHandle { node: _node, node_exit_future } = builder
                    .node(BerachainNode::default())
                    .install_exex("my_indexer", |ctx| async move { Ok(exex::my_indexer(ctx)) })
                    .launch()
                    .await?;

                node_exit_future.await
            },
        )
    {
        eprintln!("Error: {err:?}");
        std::process::exit(1);
    }

    Ok(())
}
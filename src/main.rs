//! Bera-Reth ExEx Template

use zg_reth_exex_template::exex::{self, IndexerArgs};

use clap::Parser;
use reth::cli::Cli;
use reth_ethereum_cli::chainspec::EthereumChainSpecParser;
use reth_node_ethereum::EthereumNode;

fn main() -> eyre::Result<()> {
    Cli::<EthereumChainSpecParser, IndexerArgs>::parse().run(|builder, indexer_args| async move {
        let handle = builder
            .node(EthereumNode::default())
            .install_exex("staking_indexer", move |ctx| async move {
                Ok(exex::staking_indexer_exex(ctx, indexer_args.db_path))
            })
            .launch_with_debug_capabilities()
            .await?;

        handle.wait_for_node_exit().await
    })
}

use eyre::Result;
use ethers_core::{
    abi::Abi,
    types::{Address, H256},
};

use ethers_contract::Contract;
use ethers_providers::{Provider, Http};
use ethers_signers::Wallet;

use ethers::{
    contract::{abigen, Contract},
    core::types::ValueOrArray,
    providers::{Provider, StreamExt, Ws},
    utils::Ganache,
};

use std::{
    error::Error,
    sync::Arc,
    convert::TryFrom,
};

abigen!(
    AggregatorInterface,
    r#"[
        event AnswerUpdated(int256 indexed current, uint256 indexed roundId, uint256 updatedAt)
    ]"#,
);

const PRICE_FEED_1: &str = "0x7de93682b9b5d80d45cd371f7a14f74d49b0914c";

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    // Spawn a ganache instance
    // let ganache = Ganache::new().spawn();
    // println!("HTTP Endpoint: {}", ganache.endpoint());

    let client = get_client().await;
    let client = Arc::new(client);

    // Build an Event by type. We are not tied to a contract instance. We use builder functions to
    // refine the event filter
    let event = Contract::event_of_type::<AnswerUpdatedFilter>(&client)
        .from_block(16022082)
        .address(ValueOrArray::Array(vec![
            PRICE_FEED_1.parse()?,
        ]));

    let mut stream = event.subscribe_with_meta().await?.take(2);

    // Note that `log` has type AnswerUpdatedFilter
    while let Some(Ok((log, meta))) = stream.next().await {
        println!("{log:?}");
        println!("{meta:?}")
    }

    Ok(())
}

async fn get_client() -> Provider<Ws> {
    Provider::<Ws>::connect("wss://mainnet.infura.io/ws/v3/c60b0bb42f8a4c6481ecd229eddaca27")
        .await
        .unwrap()
}
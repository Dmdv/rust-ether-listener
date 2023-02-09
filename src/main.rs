#[macro_use]
extern crate lazy_static;

use ethers::{
    contract::{abigen, Contract, EthEvent},
    core::{types::ValueOrArray},
    providers::{Provider, StreamExt, Ws},
    utils::{Ganache, GanacheInstance},
};
use std::{
    error::Error,
    sync::{Arc, Mutex},
    any::type_name,
    fmt::Debug,
};
use ethers::providers::Middleware;
use eyre::Result;

abigen!(
    FarawayNFT,
    "src/FarawayNFT.json",
    event_derives (serde::Deserialize, serde::Serialize),
);

lazy_static! {
    static ref EVENTS: Mutex<Vec<String>> = Mutex::new(vec![]);
}

const STARTING_BLOCK: i32 = 8450915;
const NFT_FEED: &str = "0xfeDB19A138fdF3432A88eB3dB9AD36f7aed073B0";
const WSS_URL: &str = "wss://goerli.infura.io/ws/v3/20d3e6b3b40f40399f1bf6c458c37974";

/// ProviderWs is a type alias for a Provider with a Websocket transport.
type ProviderWs = Provider<Ws>;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let provider = get_client().await;
    let client = Arc::new(provider);

    let version = client.client_version().await?;
    println!("Client Version: {}", version);

    // Build an Event by type. We are not tied to a contract instance.
    // We use builder functions to refine the event filter
    let event = Contract::event_of_type::<CollectionCreatedFilter>(&client)
        .from_block(STARTING_BLOCK)
        .address(ValueOrArray::Array(vec![
            NFT_FEED.parse()?,
        ]));

    let mut stream = event.subscribe_with_meta().await?.take(1);

    // Note that `log` has type AnswerUpdatedFilter
    while let Some(Ok((log, meta))) = stream.next().await {
        println!("{log:?}");
        println!("{meta:?}")
    }

    println!("Completed...");

    Ok(())
}

/// Given a contract instance subscribe to a single type of event.
///
/// # Example:
///
/// ```norun
/// const NFT_FEED: &str = "0xfeDB19A138fdF3432A88eB3dB9AD36f7aed073B0";
/// let address: Address = NFT_FEED.parse()?;
/// let contract = FarawayNFT::new(address, Arc::clone(&client));
/// listen_specific_events::<CollectionCreatedFilter>(&contract, STARTING_BLOCK).await?;
/// ```
#[allow(dead_code)]
async fn listen_specific_events<Ev: EthEvent>(contract: &Contract<ProviderWs>, block: i32) -> Result<()>
    where
        Ev: Default + Debug,
{
    let events = contract.event::<Ev>().from_block(block);
    let mut stream = events.stream().await?.take(1);

    let type_name = type_of(Ev::default());
    println!("Event type: {type_name:?}");

    while let Some(Ok(f)) = stream.next().await {
        println!("{f:?}");
    }

    Ok(())
}

/// Given a contract instance query a single type of evens without subscribing.
///
/// # Example:
///
/// ```norun
/// const NFT_FEED: &str = "0xfeDB19A138fdF3432A88eB3dB9AD36f7aed073B0";
/// let address: Address = NFT_FEED.parse()?;
/// let contract = FarawayNFT::new(address, Arc::clone(&client));
/// let logs = query_events::<CollectionCreatedFilter>(&contract, STARTING_BLOCK).await?;
/// ```
#[allow(dead_code)]
async fn query_events<Ev: EthEvent>(contract: &Contract<ProviderWs>, block: i32) -> Result<Vec<Ev>> {
    let logs = contract.event::<Ev>().from_block(block).query().await?;

    Ok(logs)
}

/// Get a client to interact with the Ethereum network.
async fn get_client() -> ProviderWs {
    ProviderWs::connect(WSS_URL)
        .await
        .unwrap()
}

/// Get a local Ganache instance.
#[allow(dead_code)]
async fn get_ganache() -> GanacheInstance {
    Ganache::new().spawn()
}

/// Get the type name of a generic type.
fn type_of<T>(_: T) -> &'static str {
    type_name::<T>()
}

#[macro_use] extern crate rocket;
#[macro_use] extern crate lazy_static;

use ethers::{
    contract::{abigen, Contract, EthEvent},
    core::{types::ValueOrArray},
    providers::{Provider, StreamExt, Ws},
    utils::{Ganache, GanacheInstance},
    contract::{ContractError, LogMeta},
    prelude::stream::EventStream,
    providers::{Middleware, SubscriptionStream},
    types::Log
};

use std::{error::Error, sync::{Arc, Mutex}, any::type_name, fmt::Debug};
use eyre::Result;
use tokio::task;
use rocket::{
    serde::{Serialize, json::Json},
    Rocket,
    Config,
    log::LogLevel,
    response::content::RawHtml,
};

abigen!(
    FarawayNFT,
    "src/FarawayNFT.json",
    event_derives (serde::Deserialize, serde::Serialize),
);

lazy_static! {
    static ref EVENTS: Mutex<Vec<String>> = Mutex::new(vec![]);
}

const ADDRESS: &'static str = "0.0.0.0";
const PORT: u16 = 9000;
const EVENTS_CAPACITY: usize = 100000;
const STARTING_BLOCK: i32 = 8450915;
const NFT_FEED: &'static str = "0xfeDB19A138fdF3432A88eB3dB9AD36f7aed073B0";
const WSS_URL: &'static str = "wss://goerli.infura.io/ws/v3/20d3e6b3b40f40399f1bf6c458c37974";

/// ProviderWs is a type alias for a Provider with a Websocket transport.
#[allow(dead_code)]
type ProviderWs = Provider<Ws>;
/// Stream is a type alias for an EventStream of a specific event type.
#[allow(dead_code)]
type Stream<'a, Ev> = EventStream<'a, SubscriptionStream<'a, Ws, Log>, (Ev, LogMeta), ContractError<ProviderWs>>;

/// Returns events from the NFT feed contract.
#[derive(Serialize)]
struct EventsResponse {
    data: Vec<String>,
}

#[get("/")]
fn events() -> Json<EventsResponse> {
    let data = EVENTS.lock().unwrap().clone();

    Json(EventsResponse { data })
}

#[get("/", format = "text/html")]
fn index() -> RawHtml<&'static str> {
    let body =
        "<!DOCTYPE html>
        <html>
            <head>
                <title>Faraway NFT Events</title>
            </head>
            <body>
            <h3>Faraway NFT Events</h3>
            <p>Visit <a href=\"/events\">Events</a> to see the events.</p>
            </body>
        </html>";

    RawHtml(body)
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let provider = get_client().await;
    let client_a = Arc::new(provider);
    let client_b =  Arc::clone(&client_a);

    let version = client_a.client_version().await?;
    println!("Client Version: {}", version);

    let task_a = task::spawn(async move {
        read_event_stream::<CollectionCreatedFilter>(&client_a, Some(EVENTS_CAPACITY)).await;
    });

    let task_b = task::spawn(async move {
        read_event_stream::<TokenMintedFilter>(&client_b, Some(EVENTS_CAPACITY)).await;
    });

    let task_c = task::spawn(async {
        create_rocket().await
    });

    tokio::select! {
        _ = task_a => (),
        _ = task_b => (),
        _ = task_c => (),
        _ = tokio::signal::ctrl_c() => {
            println!("Received Ctrl-C signal, cancelling tasks");
        }
    }

    println!("Exiting...");

    Ok(())
}

async fn create_rocket() -> Result<Rocket<rocket::Ignite>, rocket::Error> {
    let mut config = Config::default();
    config.port = PORT;
    config.address = ADDRESS.parse().expect("Invalid IP address");
    config.log_level = LogLevel::Normal;
    config.cli_colors = true;

    let _rocket = rocket::custom(config)
        .mount("/", routes![index])
        .mount("/events", routes![events])
        .launch().await?;

    Ok(_rocket)
}

/// Given a contract instance subscribe to a single type of event.
///
/// # Example:
///
/// ```norun
/// const NFT_FEED: &str = "0xfeDB19A138fdF3432A88eB3dB9AD36f7aed073B0";
/// let address: Address = NFT_FEED.parse()?;
/// let contract = FarawayNFT::new(address, Arc::clone(&client));
/// listen_specific_events::<CollectionCreatedFilter>(&contract).await?;
/// ```
#[allow(dead_code)]
async fn listen_specific_events<Ev: EthEvent>(contract: &Contract<ProviderWs>) -> Result<(), Box<dyn Error>>
    where
        Ev: Default + Debug,
{
    println!("Started reading event stream");

    let events = contract.event::<Ev>().from_block(STARTING_BLOCK);
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
/// let logs = query_events::<CollectionCreatedFilter>(&contract).await?;
/// ```
#[allow(dead_code)]
async fn query_events<Ev: EthEvent>(contract: &Contract<ProviderWs>) -> Result<Vec<Ev>> {
    let logs = contract.event::<Ev>().from_block(STARTING_BLOCK).query().await?;
    Ok(logs)
}

/// Build an Event by type, not tied to a contract instance.
/// Uses builder functions to refine the event filter
#[allow(dead_code)]
async fn read_event_stream<'a, Ev: EthEvent + 'a + Debug>(client: &'a Arc<ProviderWs>, take: Option<usize>)
{
    println!("Started reading event stream");

    let event = Contract::event_of_type::<Ev>(client)
        .from_block(STARTING_BLOCK)
        .address(ValueOrArray::Array(vec![
            NFT_FEED.parse().unwrap(),
        ]));

    let mut stream = event
        .subscribe_with_meta().await.unwrap()
        .take(take.unwrap_or(1));

    while let Some(Ok((log, meta))) = stream.next().await {
        println!("{log:?}");
        println!("{meta:?}");

        EVENTS.lock().unwrap().push(format!("{:?}", log));
    }

    println!("Completed reading event stream");
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
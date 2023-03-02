## rust-ether-listener
Listen to the contract events using rocker crate and Ether crate

Refers to:

- https://github.com/Dmdv/nft-frontend
- https://github.com/Dmdv/nft-factory

### Usage

1. Import rocket crate
2. Import ABI

```rust
abigen!(
    FarawayNFT,
    "src/FarawayNFT.json",
    event_derives (serde::Deserialize, serde::Serialize),
);
```

3. Start reading the event stream

```rust
read_event_stream::<CollectionCreatedFilter>(&client_a, Some(EVENTS_CAPACITY)).await;
```

4. Run

```shell
cargo run --release
```
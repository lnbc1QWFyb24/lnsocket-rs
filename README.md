# lnsocket-rs

A Rust library for establishing connections to Lightning Network nodes. This library provides low-level primitives for establishing encrypted communication channels with Lightning Network nodes using the Noise_XK protocol as specified in [BOLT #8](https://github.com/lightning/bolts/blob/master/08-transport.md).

## Overview

`lnsocket-rs` allows you to establish secure connections to Lightning Network nodes and exchange Lightning Network messages. It's built using components from the [Lightning Development Kit (LDK)](https://lightningdevkit.org/) and offers a lightweight, focused API for Lightning Network communication.

## Features

- [x] Establish encrypted connections to Lightning Network nodes with Noise_XK handshake protocol
- [x] Send and receive Lightning Network messages
- [ ] Support for Commando CLN RPC messages

## Dependencies

- `bitcoin` (v0.32.2) - For Bitcoin primitives and cryptography
- `lightning-types` (v0.2.0) - For Lightning Network data types
- `hashbrown` (v0.13) - For hash collections

## Usage

Add to your `Cargo.toml`:

```toml
[dependencies]
lnsocket = "0.1.0"
```

Basic example (API details may change):

```rust
use lnsocket::ln::peer_channel_encryptor::{PeerChannelEncryptor, NextNoiseStep};
use bitcoin::secp256k1::{PublicKey, SecretKey, Secp256k1};

// Create a new connection to a node
let node_id = PublicKey::from_str("02eec7245d6b7d2ccb30380bfbe2a3648cd7a942653f5aa340edcea1f283686619")?;
let ephemeral_key = SecretKey::new(&mut rand::thread_rng());
let secp_ctx = Secp256k1::new();

// .. TBD ...
```

## Status

This library is experimental and under active development. APIs may change significantly between versions.

## License

This library contains code derived from LDK, which is licensed under either of:

- Apache License, Version 2.0 ([LICENSE-APACHE](http://www.apache.org/licenses/LICENSE-2.0))
- MIT License ([LICENSE-MIT](http://opensource.org/licenses/MIT))

at your option.

## Contributing

Contributions are welcome! Please feel free to submit a Pull Request.

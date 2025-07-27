# lnsocket-rs

A Rust library for establishing connections to Lightning Network nodes. This library provides low-level primitives for establishing encrypted communication channels with Lightning Network nodes using the Noise_XK protocol as specified in [BOLT #8](https://github.com/lightning/bolts/blob/master/08-transport.md).

lnsocket also comes batteries included with Commando support, allowing you to call RPCs on core-lightning nodes remotely!

## Overview

`lnsocket-rs` allows you to establish secure connections to Lightning Network nodes and exchange Lightning Network messages. It's built using components from the [Lightning Development Kit (LDK)](https://lightningdevkit.org/) and offers a lightweight, focused API for Lightning Network communication.

## Features

- [x] Establish encrypted connections to Lightning Network nodes with Noise_XK handshake protocol
- [x] Send and receive Lightning Network messages
- [x] Support for Commando CLN RPC messages

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
use lnsocket::{LNSocket, CommandoClient};
use bitcoin::secp256k1::{PublicKey, SecretKey};

async fn test_commando() -> Result<(), Error> {
    use crate::commando::CommandoClient;

    let key = SecretKey::new(&mut rand::thread_rng());
    let their_key = PublicKey::from_str(
        "03f3c108ccd536b8526841f0a5c58212bb9e6584a1eb493080e7c1cc34f82dad71",
    )
    .unwrap();

    let mut lnsocket = LNSocket::connect_and_init(key, their_key, "ln.damus.io:9735").await?;
    let mut commando = CommandoClient::new(
        "hfYByx-RDwdBfAK-vOWeOCDJVYlvKSioVKU_y7jccZU9MjkmbWV0aG9kPWdldGluZm8=",
    );
    let resp = commando.call(&mut lnsocket, "getinfo", vec![]).await?;

    println!("{}", serde_json::to_string(&resp).unwrap());

    Ok(())
}

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

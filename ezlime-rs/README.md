# ezlime-rs

[![Crates.io](https://img.shields.io/crates/v/ezlime-rs.svg)](https://crates.io/crates/ezlime-rs)
[![Documentation](https://docs.rs/ezlime-rs/badge.svg)](https://docs.rs/ezlime-rs)

A Rust client library for the [ezli.me](https://ezli.me) URL shortener API.

## Installation

Add this to your `Cargo.toml`:

```toml
[dependencies]
ezlime-rs = "0.1"
```

## Getting an API Key

To use the ezli.me API, you'll need an API key. If you're interested in using ezli.me for your own project, please join our [Discord server](https://discord.gg/MHzmYHnnsE) to request an API key.

## Basic Example

```rust
use ezlime_rs::EzlimeApi;

#[tokio::main]
async fn main() -> Result<(), ezlime_rs::EzlimeApiError> {
    let api = EzlimeApi::new("your-api-key-here".to_string());
    let original_url = "https://example.com/very/long/url/that/needs/shortening";

    let shortened = api.create_short_url(original_url).await?;
    println!("Shortened URL: {}", shortened);
    
    Ok(())
}
```

## License

This project is licensed under either of:

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)
- MIT License ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)

at your option.

## Contributing

Contributions are welcome! Please feel free to submit a Pull Request.

## Links

- [Documentation](https://docs.rs/ezlime-rs)
- [Crates.io](https://crates.io/crates/ezlime-rs)
- [ezli.me Website](https://ezli.me)
- [Discord Community](https://discord.gg/MHzmYHnnsE)

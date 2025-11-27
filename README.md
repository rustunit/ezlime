# ezli.me

Easy-link-me link shortener: [ezli.me](https://ezli.me)

Find also an easy to use crate [ezlime-rs](https://crates.io/crates/ezlime-rs) that allows accessing the api from rust in your own service.

## Features

- Shorten URLs
- Customizable short URLs
- Click tracking
- Privacy-focused

## Hosted Service

If you are just interested in using our public API reach out to discuss your project and usage limits and we can provide you with an API key: [Join our Discord](https://discord.gg/MHzmYHnnsE)

## Design

It uses a very simple design and focuses on privacy. It does not store access information, no IPs, nothing. The only thing stored is a click counter and when a link was accessed last time.

## Dependencies

This service requires only a postgres database for storing links. It does heavy in-memory caching of both url to forward to (*DB-read*) but also updating `click_count` & `last_use` stats to limit *DB-write* load.

## Self hosting

Self hosting should be easy using our [Docker](./Dockerfile) image: `ghcr.org/rustunit/ezlime`

## Building

### get build working with diesel (on macos)

```
brew install libpq
```

follow instructions to add a bunch of `env`s:

```sh
export PATH="/opt/homebrew/opt/libpq/bin:$PATH"
export LDFLAGS="-L/opt/homebrew/opt/libpq/lib"
export CPPFLAGS="-I/opt/homebrew/opt/libpq/include"
export PKG_CONFIG_PATH="/opt/homebrew/opt/libpq/lib/pkgconfig"
```

## License

This project is licensed under either of:

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)
- MIT License ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)

at your option.

## Contributing

Contributions are welcome! Please feel free to submit a Pull Request.

## Links

- [ezli.me Website](https://ezli.me)
- [Discord Community](https://discord.gg/MHzmYHnnsE)

# HyperDeck Plugin for StreamDeck

Stream Deck plugin for any number of named HyperDeck decks.

The latest build is on [GitHub Releases](https://github.com/MikanseiLaboratory/streamdeck-bmd-hyperdeck/releases/latest).

## Requirements

- Stream Deck 6.4+
- HyperDeck Ethernet Protocol on TCP 9993
- Windows 10 or macOS 10.15+

## Use

The first launch adds `127.0.0.1:9993`. Add more decks from a key's property inspector, then choose All, a group, or individual decks on each action. Logs are written to the Stream Deck log.

## Build

```sh
cargo test --workspace
cargo run -p hyperdeck-plugin --bin typegen
cd pi && npm ci && npm run build
INSTALL=1 ./publish.sh
```

`hyperdeck-rs` is taken from the git revision in `Cargo.toml`.

Tags `v0.1.0.0`, `v0.1.0.0-alpha`, `v0.1.0.0-alpha.1`, and the same forms for `beta` and `rc`, publish a GitHub Release. Alpha, beta, and rc tags are prereleases. The four-number version must match `manifest.json`.

## License

[MIT](LICENSE-MIT) OR [Apache-2.0](LICENSE-APACHE).

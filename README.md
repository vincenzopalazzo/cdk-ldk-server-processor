# cdk-ldk-server-processor

A [CDK](https://github.com/cashubtc/cdk) gRPC **payment processor** backed by an external
[ldk-server](https://github.com/lightningdevkit/ldk-server) daemon.

It lets a **stock** `cdk-mintd` use ldk-server as its Lightning backend through the
published CDK payment-processor protocol — no cdk fork and no crates.io publishing
blocker (this binary is never published, so the git-pinned `ldk-server-client`
dependency is fine here).

The heavy lifting (`MintPayment` implementation: BOLT11 + BOLT12 mint/melt flows,
payment status lookups, event streaming) is reused directly from
[cashubtc/cdk#2164](https://github.com/cashubtc/cdk/pull/2164) via a git pin.

## Architecture

```
cdk-mintd (stock upstream, feature: grpc-processor)
   │  gRPC :50051   (CDK payment processor protocol)
   ▼
cdk-ldk-server-processor   ← this repo
   │  gRPC+TLS :3536 (ldk-server protocol, API-key auth)
   ▼
ldk-server (lightningdevkit/ldk-server)
```

## Quick start

```bash
export LDK_SERVER_ADDR=127.0.0.1:3536        # ldk-server gRPC address (no scheme)
export LDK_SERVER_API_KEY=<hmac api key>     # from ldk-server's api_key file
export LDK_SERVER_TLS_CERT=/path/to/tls.crt  # PEM cert to pin
cargo run --release
```

Then point cdk-mintd at it:

```toml
[ln]
ln_backend = "grpcprocessor"

[grpc_processor]
addr = "127.0.0.1"
port = 50051
allow_insecure = true   # localhost; configure tls_dir for production
```

(`cdk-mintd` built with `--features grpc-processor`.)

## Configuration

All configuration is via environment variables:

| Variable | Required | Default | Description |
|---|---|---|---|
| `LDK_SERVER_ADDR` | ✅ | — | ldk-server gRPC address, e.g. `127.0.0.1:3536` |
| `LDK_SERVER_API_KEY` | ✅ | — | HMAC API key expected by ldk-server |
| `LDK_SERVER_TLS_CERT` | ✅ | — | Path to PEM-encoded TLS certificate to pin |
| `SERVER_PORT` | | `50051` | Port this processor listens on |
| `FEE_RESERVE_MIN_SAT` | | `2` | Absolute fee reserve for melt quotes |
| `FEE_RESERVE_PERCENT` | | `0.01` | Relative fee reserve (0.01 = 1%) |
| `MAX_PAYMENT_SCAN_PAGES` | | `32` | Max `ListPayments` pages scanned for status lookups |

## Status

Running in production against the [Hedwig](https://mint.hedwig.sh) Cashu mint.

- [x] BOLT11 mint/melt via ldk-server
- [x] BOLT12 offers (requires the rust-lightning intro-node fix,
      [rust-lightning#4828](https://git.rust-bitcoin.org/lightningdevkit/rust-lightning/pulls/4828))
- [ ] TLS between mint and processor (currently localhost plaintext)
- [ ] Upstream the backend once ldk-server is published on crates.io
      (tracking: [cashubtc/cdk#2164](https://github.com/cashubtc/cdk/pull/2164),
      RFC [cashubtc/cdk#2170](https://github.com/cashubtc/cdk/issues/2170))

See [PLAN.md](PLAN.md) for the full design rationale and deployment phases.

## License

MIT

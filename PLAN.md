# Plan B: ldk-server as a CDK payment processor

## Why

PR cashubtc/cdk#2164 (in-tree `cdk-ldk-server` backend for cdk-mintd) is blocked on
crates.io publishing: the git-pinned `ldk-server-client` dependency would break
`cargo publish` for the workspace. Maintainer suggestion: run it as an external
**payment processor** until ldk-server is published on crates.io.

The processor binary is never published, so git dependencies are fine here.
cdk-mintd only needs the *published* `cdk-payment-processor` client
(`ln_backend = "grpcprocessor"`, `[grpc_processor] addr/port`), which already
exists upstream — no fork of cdk-mintd needed.

## Architecture

```
cdk-mintd (stock upstream, feature grpc-processor)
   |  gRPC (CDK payment processor protocol, port 50051)
   v
cdk-ldk-server-processor  <-- this repo (thin wrapper)
   |  gRPC + TLS + API key (ldk-server protocol, port 3536)
   v
ldk-server daemon (lightningdevkit/ldk-server, with our
   rust-lightning intro-node fix backport)
```

## Phases

1. **Scaffold** (this commit): settings + main wrapping
   `cdk_ldk_server::CdkLdkServer` in `PaymentProcessorServer`.
   cdk crates pinned via git to hedwig-corp/cdk `codex/propose-ldk-server-backend`
   so the MintPayment impl from PR #2164 is reused without porting 1.1k lines.
2. **Compile + local smoke test**: `cargo check`, then run against the
   production ldk-server (read-only calls first: get_settings).
3. **Mint wiring**: build stock cashubtc/cdk cdk-mintd with
   `--features grpc-processor`, config `ln_backend = "grpcprocessor"`,
   `[grpc_processor] addr = "127.0.0.1", port = 50051, allow_insecure = true`
   (localhost; TLS optional later).
4. **Deploy on 65.108.246.14**: run processor as a daemon next to ldk-server,
   switch cdk-mintd over, verify BOLT11 + BOLT12 mint/melt quotes end-to-end.
5. **Upstream story**: report the deployment on cdk#2164 / RFC cdk#2170; the
   in-tree backend PR can then wait for ldk-server on crates.io without
   blocking production.

## Config (env)

- `LDK_SERVER_ADDR` (e.g. `65.108.246.14:3536`)
- `LDK_SERVER_API_KEY` (HMAC key from ldk-server api_key file)
- `LDK_SERVER_TLS_CERT` (path to PEM cert to pin)
- `SERVER_PORT` (default 50051)
- `FEE_RESERVE_MIN_SAT` (default 2), `FEE_RESERVE_PERCENT` (default 0.01)
- `MAX_PAYMENT_SCAN_PAGES` (default 32)

## Later

- TLS between mint and processor (`tls_dir` on both sides).
- Apply the codex review fixes (P1: BOLT11 melt lookup keyed by payment hash;
  P2: scan-limit-before-matches) — either upstreamed into #2164 or patched here.
- When ldk-server hits crates.io: revisit merging #2164 as the in-tree option.

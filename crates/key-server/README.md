# Mys Seal Key Server

A cryptographic key server for the Mys Seal system that provides secure, policy-based access to encryption keys using Identity-Based Encryption (IBE).

## Overview

The key server is a core component of the Mys Seal ecosystem that:

- **Derives encryption keys** using IBE from a master secret key
- **Enforces access policies** by validating Move smart contracts on the Mys blockchain
- **Provides secure endpoints** for key retrieval with signature verification
- **Operates statelessly** - no database required, all keys derived cryptographically

## Quick Start

### Prerequisites

- Rust 1.75 or later
- Access to a Mys blockchain node (testnet/mainnet)
- Generated master key and registered key server object

### Environment Variables

```bash
export MASTER_KEY="your_base64_or_hex_encoded_master_key"
export KEY_SERVER_OBJECT_ID="0x1234...your_object_id"
export NETWORK="testnet"  # or "mainnet", "custom"
```

For custom networks, also set:
```bash
export NODE_URL="https://your-node.com:9000"
export GRAPHQL_URL="https://your-graphql.com/graphql"
```

### Build and Run

```bash
# Build the key server
cargo build --release --bin key-server

# Run the server
cargo run --bin key-server
```

The server will start on port 8080 by default (configurable via `PORT` environment variable).

## API Endpoints

### `GET /v1/service`
Returns service information and proof of possession:
```json
{
  "service_id": "0x...",
  "pop": "...",
  "version": "0.0.1"
}
```

### `POST /v1/fetch_key`
Fetch decryption keys (requires valid signature and policy compliance):
```json
{
  "ptb": "base64_encoded_programmable_transaction",
  "enc_key": "...",
  "enc_verification_key": "...",
  "request_signature": "...",
  "certificate": {
    "user": "0x...",
    "session_vk": "...",
    "creation_time": 1640995200000,
    "ttl_min": 30,
    "signature": "..."
  }
}
```

### `GET /health`
Health check endpoint (provided by mysten-service).

## How It Works

1. **Key Derivation**: Uses IBE to derive keys from master secret + key ID
2. **Policy Enforcement**: Validates access by dry-running Move smart contracts
3. **Signature Verification**: Ensures requests come from legitimate users
4. **Stateless Operation**: No persistent storage, everything derived on-demand

## Security Features

- **Non-interactive threshold signatures** for distributed key management
- **Time-limited session keys** with configurable TTL
- **Policy-based access control** enforced by blockchain smart contracts
- **Signature verification** for all key requests
- **Proof of possession** to prevent key server impersonation

## Deployment

### Local Development
```bash
cargo run --bin key-server
```

### Docker
```bash
docker build -t seal-key-server .
docker run -p 8080:8080 \
  -e MASTER_KEY="..." \
  -e KEY_SERVER_OBJECT_ID="..." \
  -e NETWORK="testnet" \
  seal-key-server
```

### Railway
See `railway.toml` for Railway-specific deployment configuration. Set environment variables in the Railway dashboard.

## Configuration

| Variable | Required | Description |
|----------|----------|-------------|
| `MASTER_KEY` | ✅ | IBE master secret key (base64/hex) |
| `KEY_SERVER_OBJECT_ID` | ✅ | Registered key server object ID |
| `NETWORK` | ✅ | Mys network (`testnet`, `mainnet`, `custom`) |
| `NODE_URL` | ⚠️ | Custom node URL (for `custom` network) |
| `GRAPHQL_URL` | ⚠️ | Custom GraphQL URL (for `custom` network) |
| `PORT` | ❌ | Server port (default: 8080) |

## Generating Keys

Use the `seal-cli` tool to generate a master key:

```bash
cargo run --bin seal-cli genkey
```

## Monitoring

The server provides:
- **Prometheus metrics** on `/metrics` endpoint
- **Health checks** on `/health` endpoint
- **Structured logging** with tracing support
- **Request/response monitoring** with timing and error metrics

## Architecture

```
┌─────────────────┐    ┌─────────────────┐    ┌─────────────────┐
│   Client App    │───▶│   Key Server    │───▶│ MySo Blockchain │
└─────────────────┘    └─────────────────┘    └─────────────────┘
                              │
                              ▼
                       ┌─────────────────┐
                       │ IBE Key Store   │
                       │ (In Memory)     │
                       └─────────────────┘
```

## License

Apache-2.0 
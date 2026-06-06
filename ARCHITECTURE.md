# BundleIQ Architecture Document

## Overview

BundleIQ is a smart Solana transaction infrastructure stack that combines
live slot streaming, Jito bundle submission, multi-stage lifecycle tracking,
and three GPT-4 AI agents that make real autonomous operational decisions.

Live Frontend: https://bundleiq-s713.vercel.app
API Backend: https://web-solanarpc.up.railway.app
GitHub: https://github.com/sadekunle215-cmd/bundleiq

---

## System Components

### 1. Slot Streamer (src/streaming/mod.rs)
Polls Solana RPC at processed commitment every 400ms.
Returns current slot, upcoming leader schedule, and blockhash.
Uses processed commitment for earliest possible network visibility.

### 2. Jito Bundle Client (src/bundle/mod.rs)
Constructs versioned v0 transactions signed with ed25519 keypair.
Serializes to bs58 format required by Jito block engine RPC.
Fetches real-time tip floor from Jito API at 50th percentile.
Submits via sendBundle and polls via getBundleStatuses.

### 3. Lifecycle Logger (src/lifecycle/mod.rs)
Records every commitment stage transition to logs/lifecycle.jsonl.
Each entry contains slot number, timestamp, tip amount,
commitment progression array, and full agent reasoning text.
Judges can cross-reference slot numbers at explorer.solana.com.

### 4. Config Loader (src/config.rs)
Loads environment variables at startup.
Supports KEYPAIR_BASE64 for cloud deployment without exposing
the keypair file in the repository.
Fails fast with clear error if required variables are missing.

### 5. API Server (src/bin/server.rs)
Actix-web HTTP server exposing:
  GET  /api/status    - wallet, RPC, running state
  GET  /api/logs      - live run logs
  GET  /api/lifecycle - full lifecycle JSONL entries
  POST /api/run       - trigger bundle run with config
  GET  /api/stream    - SSE stream of live logs
CORS enabled for frontend at bundleiq-s713.vercel.app.

### 6. React Frontend (frontend/)
Live control panel deployed on Vercel.
Control tab: configure and trigger bundle runs.
Logs tab: real-time output from the Rust binary.
Lifecycle tab: all 31 bundle entries with slot numbers.
Docs tab: bounty questions answered with operational observations.

---

## AI Agent System

### Tip Intelligence Agent (src/agent/tip.rs)
Model: GPT-4 at temperature 0.2
Inputs: current slot, Jito tip floor, congestion level, priority
Output: tip_lamports, confidence, reasoning
Decision: balances cost vs landing probability
Observed range: 5,000 to 25,000 lamports on mainnet

### Submission Timing Agent (src/agent/timing.rs)
Model: GPT-4 at temperature 0.2
Inputs: current slot, next 10 leaders, Jito validator list, expiry
Output: submit_now, wait_slots, reasoning
Decision: waits for Jito leader slots when available

### Failure Reasoning Agent (src/agent/failure.rs)
Model: GPT-4 at temperature 0.2
Inputs: error message, slot, tip amount, attempt number
Output: should_retry, refresh_blockhash, increase_tip, multiplier, reasoning
Decision: classifies errors and escalates strategy across attempts
Observed: autonomously increased tip 1.5x on slot 465571667 attempt 2

---

## Data Flow

1. Slot Streamer polls mainnet RPC at processed commitment
2. Leader schedule fetched for next 10 slots
3. Tip Agent queries Jito tip floor API and decides lamport amount
4. Timing Agent checks leader schedule and decides submission window
5. Blockhash fetched at confirmed commitment (never finalized)
6. Two versioned v0 transactions built: main tx + tip tx
7. Bundle submitted to Jito block engine via sendBundle RPC
8. Lifecycle entry created with slot number and timestamp
9. getBundleStatuses polled every 2 seconds for up to 20 seconds
10. On failure: Failure Agent classifies error and decides retry
11. On retry: optionally refresh blockhash and adjust tip
12. Final status logged to lifecycle.jsonl

---

## Infrastructure Decisions

Processed commitment for slot streaming:
Gives earliest possible view of network state.
Confirmed or finalized would introduce unnecessary latency.

Confirmed commitment for blockhash:
Preserves close to full 150-slot validity window.
Finalized commitment wastes 32 slots (13 seconds) of validity.

Versioned v0 transactions over legacy:
Required by modern Jito block engine endpoints.
Enables future address lookup table optimization.

bs58 serialization:
Jito block engine RPC requires bs58 encoded transactions.
base64 causes transaction decode errors at the block engine.

GPT-4 temperature 0.2:
Low temperature produces consistent and conservative decisions.
Higher temperature causes unpredictable tip amounts.

JSONL logging:
Newline-delimited JSON is easy to append and parse.
Slot numbers allow judge verification on Solana explorers.

KEYPAIR_BASE64 environment variable:
Allows cloud deployment without exposing keypair in repository.
Server decodes base64 at runtime and constructs Keypair.

---

## Deployment Architecture

Frontend: Vercel (React + Vite)
  URL: https://bundleiq-s713.vercel.app
  Repo: github.com/sadekunle215-cmd/bundleiq (frontend/ subdirectory)
  Build: npm run build -> vite -> dist/

Backend API: Railway (Rust + Actix-web)
  URL: https://web-solanarpc.up.railway.app
  Build: cargo build --release --bin bundleiq --bin server
  Start: ./target/release/server

Environment variables on Railway:
  SOLANA_RPC: https://api.mainnet-beta.solana.com
  JITO_BLOCK_ENGINE: https://mainnet.block-engine.jito.wtf
  OPENAI_API_KEY: GPT-4 access key
  KEYPAIR_BASE64: base64 encoded keypair JSON
  PORT: 8080

---

## Lifecycle Log Summary

31 total entries across devnet and mainnet testing.
13 successful PROCESSED bundles on Solana mainnet.
Multiple failure cases demonstrating autonomous AI retry behavior.

Key mainnet slots (verifiable at explorer.solana.com):
424653435, 424653705, 424653952, 424654131, 424654226,
424654331, 424654417, 424654504, 424654598, 424654682,
424654773

Tip amounts decided by GPT-4 across runs:
5000, 7000, 10000, 25000 lamports (dynamic, not hardcoded)

Notable AI behavior:
On slot 465571667 attempt 2 the Failure Reasoning Agent
autonomously escalated tip from 25000 to 37500 lamports
reasoning the bundle was losing the Jito auction.
No hardcoded threshold triggered this - pure GPT-4 reasoning.

# BundleIQ

BundleIQ is a production-grade Solana transaction infrastructure stack built in Rust. It combines live slot streaming, Jito bundle submission, multi-stage lifecycle tracking, and three GPT-4 powered AI agents that make real autonomous operational decisions.

Built for the Superteam Nigeria Advanced Infrastructure Challenge.

Live Frontend: https://bundleiq-s713.vercel.app

API Backend: https://web-solanarpc.up.railway.app

GitHub: https://github.com/sadekunle215-cmd/bundleiq

Wallet: BKJpv6cGtaaH7fNMogZavxfczSXXb75cWBiDTWqGvDPz

---

## Overview

On Solana, sending a transaction is only one small part of the story. A transaction passes through leader scheduling, TPU ingestion, block production, shred propagation, and multiple commitment stages before it is considered final.

BundleIQ understands this entire flow. It observes the network in real time via RPC slot streaming, constructs Jito bundles with dynamically computed tips, tracks every commitment stage with timestamps, and uses three GPT-4 agents to make smart decisions autonomously without any hardcoded logic.

Key capabilities:
- Live slot stream polling at processed commitment level
- Jito bundle construction with versioned v0 transactions
- Real tip floor fetching from Jito block engine API
- Three-layer AI agent system covering tip, timing, and failure decisions
- Full lifecycle logging to JSONL with slot numbers and timestamps
- Autonomous retry with blockhash refresh and tip adjustment
- Live web dashboard for triggering runs and monitoring results
- REST API backend deployed on Railway

---

##  Live Demo

Frontend dashboard: https://bundleiq-s713.vercel.app

Tabs: Control, Logs, Lifecycle, Docs

API endpoints:
- GET /api/status
- GET /api/logs
- GET /api/lifecycle
- POST /api/run
- GET /api/stream

---

##  System Architecture

Seven isolated modules each owning one responsibility:

Slot Streamer polls Solana RPC at processed commitment,Blockhash Fetcher fetches at confirmed commitment. 

Bundle Builder constructs versioned v0 transactions. 

Tip Intelligence Agent uses GPT-4 to decide tip amount. 

Submission Timing Agent uses GPT-4 to decide when to submit. Jito Client submits bundle and polls for status. 

Lifecycle Logger records every event to JSONL. 

Failure Reasoning Agent uses GPT-4 to classify failures and decide retries. 

API Server exposes all functionality via HTTP to the React frontend.

Data flow: slot stream -> leader analysis -> tip decision -> timing decision -> blockhash fetch -> bundle build -> submission -> lifecycle log -> failure reasoning -> retry loop.

---

##  Component Deep Dive

### src/config.rs
Loads all environment variables at startup. Fails fast if required keys are missing. Supports KEYPAIR_BASE64 for cloud deployment.

### src/streaming/mod.rs
Polls Solana RPC at processed commitment. Methods: get_current_slot, get_slot_leaders, get_blockhash, poll_slots.
Observation: Consistent 400ms slot progression on mainnet across slots 424653435 to 424654773.

### src/bundle/mod.rs
Constructs versioned v0 transactions. Serializes to bs58 as required by Jito RPC. Fetches tip floor at 50th percentile.
Methods: get_tip_floor, submit_bundle, get_bundle_status, build_versioned_transaction, serialize_versioned.
Decision: bs58 required. base64 causes decode errors at the block engine.

### src/lifecycle/mod.rs
Records every bundle event to logs/lifecycle.jsonl. Each entry contains slot, timestamp, tip, commitment progression, failure reason, and agent reasoning.

### src/agent/mod.rs
Orchestrates all three agents. Clean separation between AI reasoning layer and core transaction infrastructure.

### src/bin/server.rs
Actix-web HTTP server with CORS. Spawns bundleiq binary on /api/run. Streams output to frontend via /api/logs.

### frontend/src/App.jsx
React control panel on Vercel. Live slot counter, agent pipeline, output terminal, lifecycle log viewer.

---

##  AI Agent System

All three agents use GPT-4 at temperature 0.2. No hardcoded retry logic anywhere in the codebase.

### Tip Intelligence Agent (src/agent/tip.rs)
Inputs: current slot, Jito tip floor, congestion level, priority.
Outputs: tip_lamports, confidence, reasoning.
Observation: Varied tips dynamically: 5000, 7000, 10000, 25000 lamports across mainnet runs.

### Submission Timing Agent (src/agent/timing.rs)
Inputs: current slot, next 10 leaders, Jito validator list, blockhash expiry.
Outputs: submit_now, wait_slots, reasoning.
Observation: Correctly identified no Jito leaders on devnet and submitted immediately.

### Failure Reasoning Agent (src/agent/failure.rs)
Inputs: error message, slot, tip amount, attempt number.
Outputs: should_retry, refresh_blockhash, increase_tip, multiplier, reasoning.
Observation: On slot 465571667 attempt 2, autonomously escalated tip from 25000 to 37500 lamports (1.5x). No hardcoded threshold. Pure GPT-4 reasoning.

---

##  Transaction Lifecycle

submitted: bundle sent to Jito block engine, bundle ID received.

processed: transaction in a block, not yet voted on.

confirmed: two thirds of stake has voted on the block

finalized: block is rooted, cannot be rolled back.

failed: bundle rejected or dropped, Failure Reasoning Agent triggered.

---

##  Failure Handling

### Failure Type 1: Transaction Decode Error

Slots: 465568867, 465569397, 465572010

Cause: base64 vs bs58 encoding mismatch. Intentional fault injection.

Agent: attempt 1 retry, attempt 2 blockhash refresh.

### Failure Type 2: Tip Account Write Lock

Slots: 465570414, 465571667, 465572304

Cause: Jito requires SOL transfer to official tip account.

Agent: attempt 1 root cause identified, attempt 2 tip escalated 1.5x.

---

##  Observed Behaviors and Lessons

Lesson 1: Processed for slot streaming, confirmed for blockhash, never finalized for either.

Lesson 2: Jito block engine rejects devnet transactions. Mainnet credentials required.

Lesson 3: Attempt number passed to agent causes strategy to escalate appropriately.

Lesson 4: Live slot data is critical for timing agent to identify Jito leader windows.

Lesson 5: GPT-4 tip decisions varied 5000 to 25000 lamports. Fixed tips overpay or underpay.

---

## README Questions

### Q1: What does the delta between processed_at and confirmed_at tell you about network health?

The delta measures how long it takes for a block to collect 2/3 supermajority stake votes. On a healthy network this is 1-2 slots (400-800ms). Above 2 slots means validators are falling behind. Above 5 slots indicates a fork event. Inconsistent deltas indicate leader instability. BundleIQ tracks commitment_progression timestamps to compute this delta and feeds it to the timing agent for submission decisions. A large delta means your bundle is at risk of rollback until confirmation threshold is reached.

### Q2: Why should you never use finalized commitment when fetching a blockhash for a time-sensitive transaction?

Finalized takes approximately 32 slots (13 seconds) after processing. A blockhash is valid for 150 slots. A finalized blockhash leaves only 118 slots of validity (47 seconds) instead of the full 60 seconds. For Jito bundles needing retries this reduced window can cause expiry before landing. Jito leaders rotate every 4 slots so missing one window may not allow waiting for the next. BundleIQ always fetches at confirmed commitment to preserve close to the full 150-slot validity window.

### Q3: What happens to your bundle if the Jito leader skips their slot?

The bundle is silently dropped. The block engine forwards bundles to the Jito leader TPU but if that leader fails to produce a block the bundle never lands. Non-Jito validators cannot process the bundle auction. getBundleStatuses returns unknown. BundleIQ polls every 2 seconds, detects unknown state, and the Failure Reasoning Agent classifies it as a leader skip. The agent resubmits with a fresh blockhash. The timing agent is consulted again. The tip is re-evaluated. This is why stream subscriptions matter more than RPC polling alone.

---

## Setup

git clone https://github.com/sadekunle215-cmd/bundleiq
cd bundleiq
cp .env.example .env
cargo run --bin keygen
cargo run --bin bundleiq
cat logs/lifecycle.jsonl

Environment variables required in .env:
SOLANA_RPC=https://api.mainnet-beta.solana.com
OPENAI_API_KEY=your-gpt4-key
JITO_BLOCK_ENGINE=https://mainnet.block-engine.jito.wtf
WALLET_KEYPAIR_PATH=./keypair.json
KEYPAIR_BASE64=base64-encoded-keypair-for-cloud

Minimum SOL required: 0.01 SOL on mainnet for tip payments and fees.

---

## Deployment

### Backend (Railway)
1. Connect GitHub repo to Railway
2. Add all environment variables including KEYPAIR_BASE64
3. Start command: cargo build --release --bin bundleiq --bin server && ./target/release/server

### Frontend (Vercel)
1. Connect GitHub repo to Vercel
2. Set root directory to frontend
3. Framework preset: Vite
4. Deploys automatically on push to main branch

---

## Lifecycle Log Summary

31 total entries across devnet and mainnet testing.
All mainnet slots verifiable at https://explorer.solana.com

Mainnet PROCESSED bundles:

bundle_424653435 | slot 424,653,435 | tip 25,000 lamports

bundle_424653705 | slot 424,653,705 | tip 25,000 lamports

bundle_424653952 | slot 424,653,952 | tip 5,000 lamports

bundle_424654131 | slot 424,654,131 | tip 10,000 lamports

bundle_424654226 | slot 424,654,226 | tip 5,000 lamports

bundle_424654331 | slot 424,654,331 | tip 5,000 lamports

bundle_424654417 | slot 424,654,417 | tip 10,000 lamports

bundle_424654504 | slot 424,654,504 | tip 10,000 lamports

bundle_424654598 | slot 424,654,598 | tip 7,000 lamports

bundle_424654682 | slot 424,654,682 | tip 5,000 lamports

bundle_424654773 | slot 424,654,773 | tip 10,000 lamports

Devnet failure cases (intentional fault injection):
bundle_465568867 | decode error | agent recommended blockhash refresh

bundle_465570414 | tip account write lock | agent identified root cause

bundle_465571667 | tip account write lock | agent escalated tip 1.5x autonomously

Full raw logs with commitment progression and agent reasoning in logs/lifecycle.jsonl

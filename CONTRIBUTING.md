# Contributing to RustBill

Welcome to the team! We are thrilled to have you here.

Because we are building a mission-critical API (handling things like billing and payments), our highest priorities are predictability, testability, and strict boundaries. To achieve this, we use a specific architectural style.

This guide will walk you through exactly how we structure our code and why, so you can merge your features quickly and confidently.

🏗️ 1. Our Core Architecture: Vertical Slicing
Most legacy applications use "Horizontal Layering" (MVC), where all controllers are in one folder and all database models are in another. We do not do this.

Instead, we use Vertical Slices. We group code by Feature.

Why? * Zero Context Switching: If you are fixing a bug in refunds, you only need to look inside the refunds folder.

Minimized Blast Radius: If you completely rewrite the coupons module, there is zero risk of accidentally breaking checkout.

The Anatomy of a Slice
Every feature in our API (e.g., src/api/billing/refunds/) contains four specific layers:

schema.rs (The Contract): Defines the exact shape of incoming and outgoing data.

routes.rs (The Shell): Handles Axum HTTP requests, extracts JSON, and returns status codes.

service.rs (The Core): Contains 100% of the business logic. It knows nothing about HTTP or SQL.

repository.rs (The Port): Defines the Rust Traits (interfaces) needed to talk to the database.

🛡️ 2. The Golden Rules of Writing Code Here
To keep our codebase SOTA (State of the Art), every Pull Request must adhere to these four rules.

Rule #1: Protect the Core (Ports & Adapters)
Our business logic (service.rs) is sacred. It must never directly connect to a Postgres database or make raw HTTP calls to Stripe.

Instead, the service asks for what it needs using Traits (Ports), and Axum injects the actual implementation (Adapters) at runtime.

✅ DO: Pass Traits into your functions.

Rule #2: Parse, Don't Validate
Bad data should never reach the service.rs. We enforce data integrity at the absolute edge of our API. If a request is malformed, Axum must reject it before it even touches our business logic.

✅ DO: Use serde and validator on your structs.

Rule #3: Railway-Oriented Errors (No Panics)
We never use .unwrap(), .expect(), or generic String errors in our domain logic. A crash in billing is a catastrophic failure.

Instead, we use explicit enum types for errors, leveraging the ? operator to safely pass errors up the chain.

✅ DO: Define specific errors and map them to HTTP codes.

Rule #4: Test the Core, Mock the Edges
Because our service.rs uses Traits, you do not need to spin up a Docker container with Postgres just to test your business logic.

✅ DO: Write unit tests by implementing mock traits.

📝 3. How to Add a New Feature (Step-by-Step)
When you are assigned a new endpoint (e.g., creating a new coupons feature), follow this exact order of operations:

Create the Folder: mkdir src/api/billing/coupons

Write the Schema (schema.rs): Define what the JSON payload looks like.

Define the Traits (repository.rs): What data will your logic need to fetch or save? Write the Trait definitions here.

Write the Logic (service.rs): Write the pure business logic and the enum for the errors. Write your unit tests here using Mock implementations of your traits.

Wire the HTTP (routes.rs): Hook up the Axum handler. Map your domain errors to IntoResponse.

Implement the Adapter: Go to src/shared/db/ and write the actual SQLx query that implements your Trait.

Register the Route: Add your new Axum router to the main application state in main.rs.

✅ 4. Pre-PR Checklist
Before opening a Pull Request, please ensure you can check every box:

[ ] My feature is contained entirely within its own vertical slice directory.

[ ] My service.rs does not import axum, sqlx, or reqwest.

[ ] All incoming payloads are strictly typed and validated in schema.rs.

[ ] Domain errors are explicitly defined in an enum and mapped to HTTP status codes.

[ ] I have written unit tests for the core logic using Mock traits.

## Getting Started

1. Fork the repository
2. Clone your fork: `git clone https://github.com/<your-username>/RustBill.git`
3. Create a branch: `git checkout -b feature/my-feature`
4. Follow the [Quick Start](README.md#quick-start) to set up your dev environment

## Development Setup

### Frontend (Next.js)

```bash
bun install
bun run db:up
bun run db:push
bun run db:seed
bun dev
```

### Backend (Rust)

```bash
cd rustbill
docker compose up -d
cargo run -p rustbill-server
```

## Code Style

### Frontend
- Run `bun lint` before committing
- Follow existing TypeScript patterns and shadcn/ui conventions
- Use `"use client"` for all dashboard section components

### Rust
- Run `cargo fmt --all` to format code
- Run `cargo clippy --all-targets -- -D warnings` for linting
- Follow existing module patterns in `rustbill-core` and `rustbill-server`

## Testing

```bash
# Frontend tests
bun test

# Rust tests (sequential for DB isolation)
cd rustbill
cargo test -- --test-threads=1
```

## Pull Request Guidelines

1. Keep PRs focused on a single change
2. Update or add tests for your changes
3. Ensure all checks pass (lint, format, tests)
4. Write a clear PR description using the template
5. Reference any related issues

## Reporting Issues

- Use the [Bug Report](.github/ISSUE_TEMPLATE/bug_report.md) template for bugs
- Use the [Feature Request](.github/ISSUE_TEMPLATE/feature_request.md) template for suggestions

## Code of Conduct

Please read and follow our [Code of Conduct](CODE_OF_CONDUCT.md).

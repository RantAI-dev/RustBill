# Contributing to RustBill

Thank you for your interest in contributing to RustBill! This guide will help you get started.

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

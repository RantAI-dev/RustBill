<p align="center">
  <img src="rustbill-logo/RustBill Logo Full.png" width="200" alt="RustBill Logo" />
</p>

<h1 align="center">RustBill</h1>

<p align="center">
  Open-source billing, product &amp; license management platform built with Rust and Next.js
</p>

<p align="center">
  <a href="https://github.com/RantAI-dev/RustBill/actions/workflows/ci.yml"><img src="https://github.com/RantAI-dev/RustBill/actions/workflows/ci.yml/badge.svg" alt="CI" /></a>
  <a href="LICENSE"><img src="https://img.shields.io/badge/license-AGPL--3.0-blue.svg" alt="License: AGPL-3.0" /></a>
  <a href="https://www.rust-lang.org/"><img src="https://img.shields.io/badge/rust-1.82%2B-orange.svg" alt="Rust 1.82+" /></a>
  <a href="https://www.typescriptlang.org/"><img src="https://img.shields.io/badge/TypeScript-5-blue.svg" alt="TypeScript" /></a>
  <a href="https://nextjs.org/"><img src="https://img.shields.io/badge/Next.js-16-black.svg" alt="Next.js 16" /></a>
</p>

---

## Features

- **License Key Management** — Ed25519 cryptographic signing, online and offline verification, hardware-locked licenses
- **Billing Dashboard** — Real-time analytics, revenue tracking, and KPI metric cards
- **Pipeline Tracking** — Deal stages, probability scoring, and sales forecasting
- **Multi-Provider Payments** — Stripe, Xendit, and LemonSqueezy integrations out of the box
- **Customer Management** — Health scores, tier tracking, product assignments with JSONB license keys
- **Subscription & Invoicing** — Recurring billing, dunning, credit notes, and coupon management
- **Forecasting & Reports** — Revenue projections, trend analysis, and exportable reports
- **Authentication** — Built-in auth with optional Keycloak SSO support
- **API Key Management** — Secure API access with rate limiting (Governor)

## Screenshots

<!-- TODO: Add screenshots of the dashboard -->

## Quick Start

### Prerequisites

- [Bun](https://bun.sh/) (package manager and runtime)
- [Docker](https://www.docker.com/) (for PostgreSQL)
- [Rust 1.82+](https://rustup.rs/) (for the backend)

### Setup

```bash
# Clone the repository
git clone https://github.com/RantAI-dev/RustBill.git
cd RustBill

# Install frontend dependencies
bun install

# Start PostgreSQL
bun run db:up

# Push database schema
bun run db:push

# Seed initial data
bun run db:seed

# Start the frontend dev server
bun dev
```

### Rust Backend

```bash
cd rustbill

# Start the backend database (if not already running)
docker compose up -d

# Run the server
cargo run -p rustbill-server

# Run tests
cargo test -- --test-threads=1
```

The frontend runs on `http://localhost:3000` and the Rust backend on `http://localhost:3001`.

## Architecture

```
┌─────────────────────┐     ┌─────────────────────┐
│   Next.js 16 SPA    │────▶│   Rust Axum API      │
│   (Bun runtime)     │     │   (rustbill-server)   │
│   Port 3000         │     │   Port 3001           │
└────────┬────────────┘     └────────┬──────────────┘
         │                           │
         └───────────┬───────────────┘
                     ▼
          ┌─────────────────────┐
          │   PostgreSQL 17     │
          │   Port 5433         │
          └─────────────────────┘
```

The Next.js frontend proxies API calls to the Rust backend via `next.config.mjs` rewrites. Both stacks share the same PostgreSQL database.

## Tech Stack

| Layer | Technology |
|-------|-----------|
| **Frontend** | Next.js 16, React 19, TypeScript, Tailwind CSS v4, shadcn/ui |
| **Backend** | Rust, Axum 0.8, SQLx, Tower |
| **Database** | PostgreSQL 17, Drizzle ORM (frontend), SQLx (backend) |
| **Payments** | Stripe, Xendit, LemonSqueezy |
| **Auth** | Built-in + Keycloak SSO |
| **Crypto** | Ed25519-Dalek, Argon2, Bcrypt, HMAC-SHA256 |
| **Email** | Resend (frontend), Lettre (backend) |
| **Charts** | Recharts |
| **Testing** | Vitest + Testing Library (frontend), axum-test (backend) |
| **CI/CD** | GitHub Actions |
| **Runtime** | Bun, Docker (multi-stage builds) |

## Documentation

- [License Integration Guide](docs/license-integration-guide.md) — Comprehensive guide for integrating RustBill license verification into your applications (Node.js, Python, Go, C#)

## Development

```bash
# Frontend
bun dev              # Dev server
bun lint             # ESLint
bun test             # Vitest
bun run build        # Production build
bun run db:studio    # Drizzle Studio GUI

# Rust backend (from rustbill/ directory)
cargo fmt --all      # Format
cargo clippy --all-targets -- -D warnings  # Lint
cargo test -- --test-threads=1             # Test
```

## Contributing

Contributions are welcome! Please read our [Contributing Guide](CONTRIBUTING.md) and [Code of Conduct](CODE_OF_CONDUCT.md) before submitting a PR.

## License

This project is licensed under the [GNU Affero General Public License v3.0](LICENSE).

## Star History

<a href="https://star-history.com/#RantAI-dev/RustBill&Date">
 <picture>
   <source media="(prefers-color-scheme: dark)" srcset="https://api.star-history.com/svg?repos=RantAI-dev/RustBill&type=Date&theme=dark" />
   <source media="(prefers-color-scheme: light)" srcset="https://api.star-history.com/svg?repos=RantAI-dev/RustBill&type=Date" />
   <img alt="Star History Chart" src="https://api.star-history.com/svg?repos=RantAI-dev/RustBill&type=Date" />
 </picture>
</a>

---

<p align="center">
  Built with &#x1F980; by <a href="https://github.com/RantAI-dev">RantAI</a>
</p>

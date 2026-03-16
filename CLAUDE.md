# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

RantAI Billing - A Next.js 16 billing, product, and license management dashboard with analytics, pipeline tracking, and license key management. Built with TypeScript, Tailwind CSS v4, and shadcn/ui components.

## Development Commands

**Package Manager & Runtime**: This project uses `bun` exclusively (both package manager and runtime via `--bun` flag).

```bash
bun dev        # Start dev server on http://localhost:3000 (uses Bun runtime)
bun run build  # Build for production
bun start      # Start production server
bun lint       # Run ESLint
```

### Database Commands

```bash
bun run db:up      # Start PostgreSQL via Docker Compose
bun run db:down    # Stop PostgreSQL
bun run db:push    # Push schema changes to database
bun run db:seed    # Seed database with initial data
bun run db:studio  # Open Drizzle Studio GUI
```

## Architecture

### Application Structure

This is a **single-page application** using Next.js App Router with client-side section navigation:

- **Main Page** ([app/page.tsx](app/page.tsx)): Client component managing routing via state. The `activeSection` state controls which section component is rendered.
- **Sections**: All dashboard sections live in [components/dashboard/sections/](components/dashboard/sections/) (overview, pipeline, deals, customers, licenses, products, forecasting, reports, settings).
- **Navigation**: [Sidebar](components/dashboard/sidebar.tsx) component controls section changes by calling `onSectionChange` callback.
- **Layout**: Fixed sidebar with collapsible state, header bar, and main content area with animated transitions.

### Component Organization

```
components/
├── dashboard/
│   ├── sections/          # Main section components (Overview, Pipeline, Deals, etc.)
│   ├── charts/            # Recharts-based data visualizations
│   ├── header.tsx         # Top navigation bar
│   ├── sidebar.tsx        # Left navigation sidebar
│   ├── metric-card.tsx    # Reusable KPI cards
│   ├── recent-deals.tsx   # Deal list component
│   └── top-performers.tsx # Performance leaderboard
└── ui/                    # shadcn/ui components (New York style)
```

### Data Layer

**Database**: PostgreSQL 17 via Docker Compose (port 5433), managed with Drizzle ORM + postgres.js driver.

**ORM & Schema** ([lib/db/schema.ts](lib/db/schema.ts)):
- `products` — single-table inheritance for licensed/saas/api types (type-specific fields nullable, discriminated by `productType` enum)
- `customers` — core customer info with tier/trend enums
- `customer_products` — junction table with JSONB `licenseKeys` and optional metrics
- `deals` — deal tracking with product reference, usage metric fields
- `licenses` — keyed by license key string, references customer + product
- `pipeline_deals` — separate from deals (different shape: stage/probability/daysInStage)

**Validation**: Zod schemas in [lib/validations/](lib/validations/) per entity, using `drizzle-zod` patterns.

**API Routes** (all under [app/api/](app/api/)):
- `products/` — GET (list), POST; `products/[id]/` — GET, PUT, DELETE
- `deals/` — GET (filterable by status/type), POST; `deals/[id]/` — GET, PUT, DELETE
- `customers/` — GET (with joined products), POST; `customers/[id]/` — GET, PUT, DELETE
- `licenses/` — GET, POST; `licenses/[key]/` — PUT, DELETE
- `pipeline/` — GET (grouped by stage), POST; `pipeline/[id]/` — PUT, DELETE

**Client Data Fetching** ([hooks/use-api.ts](hooks/use-api.ts)):
- SWR hooks: `useProducts`, `useDeals`, `useCustomers`, `useLicenses`, `usePipeline`
- Mutation helpers: `createProduct`, `updateDeal`, `deleteLicense`, etc.
- Components call `mutate()` after mutations to revalidate

Key data types and utilities:
- [lib/product-types.ts](lib/product-types.ts): ProductType enum (`licensed`, `saas`, `api`) with styling configs
- [lib/license-keys.ts](lib/license-keys.ts): License key generation and status management
- [lib/utils.ts](lib/utils.ts): Utility functions including `cn()` for class merging

### Styling System

**Tailwind CSS v4** with custom design tokens:

- Color system: OKLCH color space for perceptual uniformity
- Design tokens: Defined in [app/globals.css](app/globals.css) as CSS variables
- Theme: Dark theme with custom sidebar, chart colors, and accent (emerald green)
- Animations: Uses `tw-animate-css` package for transitions
- Component styling: Follows shadcn/ui patterns with `class-variance-authority`

### TypeScript Configuration

- **Path alias**: `@/*` maps to project root
- **Strict mode**: Enabled, but Next.js config has `ignoreBuildErrors: true`
- When adding new files, use the established patterns and maintain type safety

### Adding New Features

1. **New Section**: Create component in `components/dashboard/sections/`, add to `Section` type in [app/page.tsx](app/page.tsx), update sidebar `navItems` array
2. **New Chart**: Use Recharts library, follow patterns in `components/dashboard/charts/`
3. **New UI Component**: Use shadcn/ui CLI or follow existing patterns in `components/ui/`
4. **New Data Type**: Add to appropriate file in `lib/` directory

### Component Patterns

- All dashboard sections are **client components** (`"use client"`)
- Use lucide-react for icons
- Prefer composition over prop drilling
- Use metric cards with staggered animations (see `delay` prop in OverviewSection)
- Follow the established product type badge color scheme
- CRUD form dialogs are inline components within each section file (not separate files)
- Use `toast` from sonner for success/error notifications
- Skeleton loading states via `@/components/ui/skeleton` during SWR fetch

### First-Time Setup

1. `bun install` — install dependencies
2. `bun run db:up` — start PostgreSQL container
3. `bun run db:push` — create database tables
4. `bun run db:seed` — populate initial data
5. `bun dev` — start dev server

### Known Configuration Notes

- Images are unoptimized (`next.config.mjs`)
- TypeScript build errors are ignored in production builds
- Uses Google Fonts: DM Sans and JetBrains Mono
- Includes Vercel Analytics

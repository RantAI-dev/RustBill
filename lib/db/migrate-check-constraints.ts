/**
 * One-time migration: Add sequences, CHECK constraints, and partial unique indexes.
 * Run with: bun run db:migrate-checks
 */
import postgres from "postgres";

const DATABASE_URL = process.env.DATABASE_URL ?? "postgres://rantai_billing:rantai_billing_dev@localhost:5433/rantai_billing";
const sql = postgres(DATABASE_URL);

const statements = [
  // ---- Sequences for invoice/credit-note numbers ----
  `CREATE SEQUENCE IF NOT EXISTS invoice_number_seq`,
  `CREATE SEQUENCE IF NOT EXISTS credit_note_number_seq`,

  // Sync sequences to current max values
  `SELECT setval('invoice_number_seq', GREATEST(COALESCE((SELECT MAX(CAST(SPLIT_PART(invoice_number, '-', 3) AS INTEGER)) FROM invoices WHERE invoice_number LIKE 'INV-%'), 0), 1))`,
  `SELECT setval('credit_note_number_seq', GREATEST(COALESCE((SELECT MAX(CAST(SPLIT_PART(credit_note_number, '-', 3) AS INTEGER)) FROM credit_notes WHERE credit_note_number LIKE 'CN-%'), 0), 1))`,

  // ---- Partial unique index for payment idempotency ----
  `CREATE UNIQUE INDEX IF NOT EXISTS payments_stripe_pi_unique ON payments (stripe_payment_intent_id) WHERE stripe_payment_intent_id IS NOT NULL`,

  // ---- CHECK constraints ----
  `DO $$ BEGIN ALTER TABLE payments ADD CONSTRAINT payments_amount_positive CHECK (amount > 0); EXCEPTION WHEN duplicate_object THEN NULL; END $$`,
  `DO $$ BEGIN ALTER TABLE refunds ADD CONSTRAINT refunds_amount_positive CHECK (amount > 0); EXCEPTION WHEN duplicate_object THEN NULL; END $$`,
  `DO $$ BEGIN ALTER TABLE invoices ADD CONSTRAINT invoices_total_non_negative CHECK (total >= 0); EXCEPTION WHEN duplicate_object THEN NULL; END $$`,
  `DO $$ BEGIN ALTER TABLE credit_notes ADD CONSTRAINT credit_notes_amount_positive CHECK (amount > 0); EXCEPTION WHEN duplicate_object THEN NULL; END $$`,
  `DO $$ BEGIN ALTER TABLE coupons ADD CONSTRAINT coupons_discount_positive CHECK (discount_value > 0); EXCEPTION WHEN duplicate_object THEN NULL; END $$`,
  `DO $$ BEGIN ALTER TABLE subscriptions ADD CONSTRAINT subscriptions_period_order CHECK (current_period_start < current_period_end); EXCEPTION WHEN duplicate_object THEN NULL; END $$`,
  `DO $$ BEGIN ALTER TABLE usage_events ADD CONSTRAINT usage_events_value_non_negative CHECK (value >= 0); EXCEPTION WHEN duplicate_object THEN NULL; END $$`,
];

async function migrate() {
  console.log("Adding sequences, CHECK constraints, and partial unique indexes...\n");

  for (const query of statements) {
    try {
      await sql.unsafe(query);
      const short = query.length > 100 ? query.slice(0, 100) + "..." : query;
      console.log(`  ✓ ${short}`);
    } catch (err) {
      const short = query.length > 100 ? query.slice(0, 100) + "..." : query;
      console.error(`  ✗ ${short}`);
      console.error(`    ${err instanceof Error ? err.message : err}`);
    }
  }

  console.log("\nDone.");
  await sql.end();
}

migrate();

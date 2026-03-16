/**
 * One-time migration: Convert all monetary columns from `real` (float4) to `numeric(12,2)`.
 * Run with: bun run db:migrate-money
 */
import postgres from "postgres";

const DATABASE_URL = process.env.DATABASE_URL ?? "postgres://rantai_billing:rantai_billing_dev@localhost:5433/rantai_billing";
const sql = postgres(DATABASE_URL);

const alterations = [
  // products
  `ALTER TABLE products ALTER COLUMN revenue TYPE numeric(12,2)`,
  `ALTER TABLE products ALTER COLUMN target TYPE numeric(12,2)`,
  `ALTER TABLE products ALTER COLUMN "change" TYPE numeric(12,2)`,
  `ALTER TABLE products ALTER COLUMN churn_rate TYPE numeric(12,4)`,
  `ALTER TABLE products ALTER COLUMN avg_latency TYPE numeric(12,4)`,
  // customers
  `ALTER TABLE customers ALTER COLUMN total_revenue TYPE numeric(12,2)`,
  // deals
  `ALTER TABLE deals ALTER COLUMN value TYPE numeric(12,2)`,
  // pricing_plans
  `ALTER TABLE pricing_plans ALTER COLUMN base_price TYPE numeric(12,2)`,
  `ALTER TABLE pricing_plans ALTER COLUMN unit_price TYPE numeric(12,2)`,
  // invoices
  `ALTER TABLE invoices ALTER COLUMN subtotal TYPE numeric(12,2)`,
  `ALTER TABLE invoices ALTER COLUMN tax TYPE numeric(12,2)`,
  `ALTER TABLE invoices ALTER COLUMN total TYPE numeric(12,2)`,
  // invoice_items
  `ALTER TABLE invoice_items ALTER COLUMN quantity TYPE numeric(12,2)`,
  `ALTER TABLE invoice_items ALTER COLUMN unit_price TYPE numeric(12,2)`,
  `ALTER TABLE invoice_items ALTER COLUMN amount TYPE numeric(12,2)`,
  // payments
  `ALTER TABLE payments ALTER COLUMN amount TYPE numeric(12,2)`,
  // usage_events
  `ALTER TABLE usage_events ALTER COLUMN value TYPE numeric(12,4)`,
  // credit_notes
  `ALTER TABLE credit_notes ALTER COLUMN amount TYPE numeric(12,2)`,
  // credit_note_items
  `ALTER TABLE credit_note_items ALTER COLUMN quantity TYPE numeric(12,2)`,
  `ALTER TABLE credit_note_items ALTER COLUMN unit_price TYPE numeric(12,2)`,
  `ALTER TABLE credit_note_items ALTER COLUMN amount TYPE numeric(12,2)`,
  // coupons
  `ALTER TABLE coupons ALTER COLUMN discount_value TYPE numeric(12,2)`,
  // refunds
  `ALTER TABLE refunds ALTER COLUMN amount TYPE numeric(12,2)`,
];

async function migrate() {
  console.log("Migrating monetary columns from real to numeric(12,2)...\n");

  for (const query of alterations) {
    try {
      await sql.unsafe(query);
      console.log(`  ✓ ${query}`);
    } catch (err) {
      console.error(`  ✗ ${query}`);
      console.error(`    ${err instanceof Error ? err.message : err}`);
    }
  }

  console.log("\nDone.");
  await sql.end();
}

migrate();

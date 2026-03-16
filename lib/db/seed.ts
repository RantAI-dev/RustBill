import { db } from "./index";
import { users } from "./schema";
import { hashPassword } from "../auth";
import { sql } from "drizzle-orm";

async function seed() {
  console.log("Seeding database...");

  // Clear users table only
  await db.execute(sql`TRUNCATE sessions, users CASCADE`);

  // ---- Users ----
  const adminPassword = await hashPassword("admin123");
  await db.insert(users).values([
    {
      id: "admin-1",
      email: "evan@rantai.com",
      name: "Evan",
      passwordHash: adminPassword,
      role: "admin" as const,
    },
  ]);
  console.log("  Admin login: evan@rantai.com / admin123");

  console.log("Seed complete!");
  process.exit(0);
}

seed().catch((e) => {
  console.error("Seed failed:", e);
  process.exit(1);
});

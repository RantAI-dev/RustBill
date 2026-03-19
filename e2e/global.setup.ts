import { chromium, expect, type FullConfig } from "@playwright/test";
import path from "node:path";
import fs from "node:fs/promises";

async function globalSetup(config: FullConfig) {
  const primaryEmail = process.env.E2E_ADMIN_EMAIL ?? "admin@rustbill.local";
  const password = process.env.E2E_ADMIN_PASSWORD ?? "admin123";
  const credentialCandidates = [
    { email: primaryEmail, password },
    { email: "admin@rustbill.local", password: "admin123" },
    { email: "evan@rantai.com", password: "admin123" },
  ];
  const baseURL = config.projects[0]?.use?.baseURL ?? "http://127.0.0.1:3000";
  const authFile = path.resolve("playwright/.auth/admin.json");

  await fs.mkdir(path.dirname(authFile), { recursive: true });

  const browser = await chromium.launch();
  const page = await browser.newPage({ baseURL: String(baseURL) });

  let loggedIn = false;
  let lastError = "unknown";
  for (const candidate of credentialCandidates) {
    for (let attempt = 1; attempt <= 2; attempt++) {
      await page.goto("/login");
      await page.getByLabel("Email").fill(candidate.email);
      await page.getByLabel("Password").fill(candidate.password);
      await page.getByRole("button", { name: /sign in/i }).click();

      try {
        await expect(page).toHaveURL(/\/$/, { timeout: 10_000 });
        loggedIn = true;
        break;
      } catch (err) {
        lastError = err instanceof Error ? err.message : String(err);
        await page.waitForTimeout(1_200 * attempt);
      }
    }

    if (loggedIn) {
      break;
    }
  }

  if (!loggedIn) {
    throw new Error(`E2E global login failed after retries: ${lastError}`);
  }

  await page.context().storageState({ path: authFile });

  await browser.close();
}

export default globalSetup;

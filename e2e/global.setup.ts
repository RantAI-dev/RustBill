import { chromium, expect, type FullConfig } from "@playwright/test";
import path from "node:path";
import fs from "node:fs/promises";

async function globalSetup(config: FullConfig) {
  const email = process.env.E2E_ADMIN_EMAIL ?? "evan@rantai.com";
  const password = process.env.E2E_ADMIN_PASSWORD ?? "admin123";
  const baseURL = config.projects[0]?.use?.baseURL ?? "http://127.0.0.1:3000";
  const authFile = path.resolve("playwright/.auth/admin.json");

  await fs.mkdir(path.dirname(authFile), { recursive: true });

  const browser = await chromium.launch();
  const page = await browser.newPage({ baseURL: String(baseURL) });

  let loggedIn = false;
  let lastError = "unknown";
  for (let attempt = 1; attempt <= 3; attempt++) {
    await page.goto("/login");
    await page.getByLabel("Email").fill(email);
    await page.getByLabel("Password").fill(password);
    await page.getByRole("button", { name: /sign in/i }).click();

    try {
      await expect(page).toHaveURL(/\/$/, { timeout: 10_000 });
      loggedIn = true;
      break;
    } catch (err) {
      lastError = err instanceof Error ? err.message : String(err);
      await page.waitForTimeout(1_500 * attempt);
    }
  }

  if (!loggedIn) {
    throw new Error(`E2E global login failed after retries: ${lastError}`);
  }

  await page.context().storageState({ path: authFile });

  await browser.close();
}

export default globalSetup;

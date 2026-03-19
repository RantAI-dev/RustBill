import { defineConfig, devices } from "@playwright/test";

const databaseUrl =
  process.env.DATABASE_URL ??
  "postgresql://rantai_billing:rantai_billing_dev@localhost:5444/rantai_billing";

export default defineConfig({
  testDir: "./e2e",
  fullyParallel: false,
  forbidOnly: !!process.env.CI,
  retries: process.env.CI ? 1 : 0,
  workers: 1,
  reporter: process.env.CI
    ? [["html", { outputFolder: "playwright-report", open: "never" }], ["list"]]
    : [["list"]],
  timeout: 90_000,
  expect: {
    timeout: 15_000,
  },
  use: {
    baseURL: process.env.PLAYWRIGHT_BASE_URL ?? "http://127.0.0.1:3000",
    trace: "retain-on-failure",
    video: "retain-on-failure",
    screenshot: "only-on-failure",
    storageState: "playwright/.auth/admin.json",
  },
  globalSetup: "./e2e/global.setup.ts",
  webServer: [
    {
      command: "cargo run -p rustbill-server",
      cwd: "./rustbill",
      url: "http://127.0.0.1:8787/health",
      timeout: 420_000,
      reuseExistingServer: !process.env.CI,
      env: {
        ...process.env,
        RUN_MODE: process.env.RUN_MODE ?? "development",
        DATABASE_URL: databaseUrl,
      },
    },
    {
      command: "bun dev",
      cwd: ".",
      url: "http://127.0.0.1:3000/login",
      timeout: 180_000,
      reuseExistingServer: !process.env.CI,
      env: {
        ...process.env,
        RUST_BACKEND_URL: process.env.RUST_BACKEND_URL ?? "http://127.0.0.1:8787",
      },
    },
  ],
  projects: [
    {
      name: "chromium",
      use: { ...devices["Desktop Chrome"] },
    },
  ],
});

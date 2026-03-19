import { expect, test } from "@playwright/test";
import { gotoSection } from "./utils";

test("@smoke login state lands on dashboard", async ({ page }) => {
  await page.goto("/");
  await expect(page.getByText("Dashboard /", { exact: false })).toBeVisible();
});

test("@core command palette and section navigation work", async ({ page }) => {
  await page.goto("/");

  await page.getByRole("button", { name: /search\.\.\./i }).click();
  await expect(page.getByText("Search", { exact: true })).toBeVisible();
  await page.getByRole("button", { name: /close/i }).click();

  await gotoSection(page, "Invoices");
  await expect(page.getByRole("heading", { name: "Invoices" })).toBeVisible();

  await gotoSection(page, "Dashboard");
  await expect(page.getByText("Dashboard /", { exact: false })).toBeVisible();
});

test("@smoke logout works", async ({ page }) => {
  await page.goto("/");
  await expect(page).toHaveURL(/\/$/);
  await page.keyboard.press("Escape");
  await page.locator("header div.relative").last().getByRole("button").first().click();
  await page.getByRole("button", { name: /sign out/i }).click();
  await expect(page).toHaveURL(/\/login/);
});

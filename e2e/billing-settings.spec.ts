import { expect, test } from "@playwright/test";
import { gotoSection, uniqueName, waitForToast } from "./utils";

test("@core settings security flows and billing portal load", async ({ page }) => {
  await page.goto("/");

  await gotoSection(page, "Settings");
  await page.getByRole("tab", { name: /api keys/i }).click();

  const keyName = uniqueName("api-key");
  await page.getByRole("button", { name: /create key/i }).click();
  await page.getByLabel("Name").fill(keyName);
  await page.getByRole("button", { name: /create key/i }).last().click();
  await waitForToast(page);
  await expect(page.getByRole("dialog").filter({ hasText: /api key created/i })).toBeVisible();
  await page.getByRole("button", { name: /done/i }).click();
  await expect(page.getByText(keyName)).toBeVisible();

  await page.getByRole("button", { name: /revoke/i }).first().click();
  await page.getByRole("button", { name: /delete|confirm|yes/i }).first().click();
  await waitForToast(page);

  await page.getByRole("tab", { name: /license signing/i }).click();
  await page.getByRole("button", { name: /generate keypair|regenerate/i }).click();
  const confirmDialog = page.getByRole("dialog").filter({ hasText: /generate|regenerate/i }).last();
  if (await confirmDialog.isVisible()) {
    await confirmDialog.getByRole("button", { name: /generate|regenerate|confirm/i }).first().click();
  }
  await waitForToast(page);

  await gotoSection(page, "Billing Portal");
  await expect(page.getByText(/credits:/i)).toBeVisible();
  await page.getByRole("main").getByRole("button", { name: /^subscriptions$/i }).click();
  await expect(page.getByText(/subscriptions/i).first()).toBeVisible();
});

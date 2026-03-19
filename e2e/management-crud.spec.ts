import { expect, test } from "@playwright/test";
import {
  confirmDeleteDialog,
  fillLabeledInput,
  gotoSection,
  openRowMenuByText,
  selectByLabel,
  uniqueName,
  waitForToast,
} from "./utils";

test("@core management CRUD path stays healthy", async ({ page }) => {
  const productName = uniqueName("product");
  const productNameUpdated = `${productName}-u`;
  const customerName = uniqueName("customer");
  const customerNameUpdated = `${customerName}-u`;
  const planName = uniqueName("plan");
  const planNameUpdated = `${planName}-u`;

  await page.goto("/");

  // Products: create + update
  await gotoSection(page, "Products");
  await page.getByRole("button", { name: /add product/i }).click();
  const productDialog = page.getByRole("dialog").last();
  await productDialog.locator("input").first().fill(productName);
  await page.getByRole("button", { name: /^platform$/i }).click();
  await productDialog.locator("input[type='number']").first().fill("25000");
  await page.getByRole("button", { name: /create product/i }).click();
  await waitForToast(page);
  await expect(page.getByRole("row").filter({ hasText: productName })).toBeVisible();

  await openRowMenuByText(page, productName);
  await page.getByRole("menuitem", { name: /edit/i }).click();
  await fillLabeledInput(page, "Name", productNameUpdated);
  await page.getByRole("button", { name: /save changes/i }).click();
  await waitForToast(page);
  await expect(page.getByRole("row").filter({ hasText: productNameUpdated })).toBeVisible();

  // Customers: create + update
  await gotoSection(page, "Customers");
  await page.getByRole("button", { name: /add customer/i }).click();
  await fillLabeledInput(page, "Company Name", customerName);
  await fillLabeledInput(page, "Industry", "Software");
  await page.getByRole("button", { name: /^growth$/i }).click();
  await fillLabeledInput(page, "Location", "Jakarta");
  await fillLabeledInput(page, "Contact", "E2E Tester");
  await fillLabeledInput(page, "Email", `${customerName}@example.com`);
  await fillLabeledInput(page, "Phone", "+62000123456");
  await page.getByRole("button", { name: /create customer/i }).click();
  await waitForToast(page);
  await expect(page.getByRole("row").filter({ hasText: customerName })).toBeVisible();

  await openRowMenuByText(page, customerName);
  await page.getByRole("menuitem", { name: /edit/i }).click();
  await fillLabeledInput(page, "Company Name", customerNameUpdated);
  await page.getByRole("button", { name: /save changes/i }).click();
  await waitForToast(page);
  await expect(page.getByRole("row").filter({ hasText: customerNameUpdated })).toBeVisible();

  // Plans: create + update
  await gotoSection(page, "Pricing Plans");
  await page.getByRole("button", { name: /new plan/i }).click();
  await fillLabeledInput(page, "Name", planName);
  await selectByLabel(page, "Billing Cycle", "monthly");
  await selectByLabel(page, "Pricing Model", "flat");
  await fillLabeledInput(page, "Base Price ($)", "99");
  await page.getByRole("button", { name: /^create$/i }).click();
  await waitForToast(page);
  await expect(page.getByRole("row").filter({ hasText: planName })).toBeVisible();

  await openRowMenuByText(page, planName);
  await page.getByRole("menuitem", { name: /edit/i }).click();
  await fillLabeledInput(page, "Name", planNameUpdated);
  await page.getByRole("button", { name: /^save$/i }).click();
  await waitForToast(page);
  await expect(page.getByRole("row").filter({ hasText: planNameUpdated })).toBeVisible();

  // Subscriptions: create + update per-sub pre-renewal days
  await gotoSection(page, "Subscriptions");
  await page.getByRole("button", { name: /new subscription/i }).click();
  await page.locator("label:has-text('Customer')").locator("..").locator("select").first().selectOption({ label: customerNameUpdated });
  const planSelect = page.locator("label:has-text('Plan')").locator("..").locator("select").first();
  const planValue = await planSelect.locator("option").evaluateAll((options, name) => {
    const byName = options.find((opt) => (opt.textContent ?? "").includes(name as string));
    if (byName) return byName.getAttribute("value") ?? "";
    const firstNonEmpty = options.find((opt) => (opt.getAttribute("value") ?? "") !== "");
    return firstNonEmpty?.getAttribute("value") ?? "";
  }, planNameUpdated);
  await planSelect.selectOption(planValue);
  await fillLabeledInput(page, "Pre-renewal Invoice Lead (days)", "3");
  await page.getByRole("button", { name: /^create$/i }).click();
  await waitForToast(page);
  await expect(page.getByRole("row").filter({ hasText: "3d" }).first()).toBeVisible();

  await openRowMenuByText(page, "3d");
  await page.getByRole("menuitem", { name: /edit/i }).click();
  await fillLabeledInput(page, "Pre-renewal Invoice Lead (days)", "5");
  await page.getByRole("button", { name: /^save$/i }).click();
  await waitForToast(page);
  await expect(page.getByRole("row").filter({ hasText: "5d" }).first()).toBeVisible();

  // Invoices: table load sanity (manual create/edit covered in dedicated billing spec)
  await gotoSection(page, "Invoices");
  await expect(page.getByRole("heading", { name: "Invoices" })).toBeVisible();

  // Cleanup created records (reverse order)
  await page.keyboard.press("Escape");

  await gotoSection(page, "Subscriptions");
  await openRowMenuByText(page, "5d");
  await page.getByRole("menuitem", { name: /delete/i }).click();
  await confirmDeleteDialog(page);
  await waitForToast(page);
});

import { expect, test } from "@playwright/test";
import { gotoSection, uniqueName, waitForToast } from "./utils";

test("@core invoice create update and payment flow works", async ({ page }) => {
  const customerName = uniqueName("invoice-customer");

  await page.goto("/");

  const createCustomer = await page.request.post("/api/customers", {
    data: {
      name: customerName,
      industry: "Software",
      tier: "Growth",
      location: "Jakarta",
      contact: "E2E Billing",
      email: `${customerName}@example.com`,
      phone: "+62000111111",
    },
  });
  expect(createCustomer.ok()).toBeTruthy();

  await gotoSection(page, "Invoices");
  await page.getByTestId("new-invoice-button").click();

  const customerSelect = page.getByTestId("invoice-form-customer");
  const customerValue = await customerSelect.locator("option").evaluateAll((options, name) => {
    const exact = options.find((opt) => (opt.textContent ?? "").trim() === (name as string));
    if (!exact) return "";
    return exact.getAttribute("value") ?? "";
  }, customerName);
  expect(customerValue).not.toBe("");
  await customerSelect.selectOption(customerValue);

  await page.getByTestId("invoice-item-description-0").fill("E2E invoice line item");
  await page.getByTestId("invoice-item-qty-0").fill("2");
  await page.getByTestId("invoice-item-price-0").fill("75");
  const createRequestPromise = page.waitForRequest((request) =>
    request.url().includes("/api/billing/invoices") && request.method() === "POST",
  );
  const createResponsePromise = page.waitForResponse((response) =>
    response.url().includes("/api/billing/invoices") && response.request().method() === "POST",
  );
  await page.getByTestId("invoice-form-submit").click({ force: true });
  const createRequest = await createRequestPromise;
  const createResponse = await createResponsePromise;
  const createPayload = createRequest.postDataJSON();
  const createBodyText = await createResponse.text();
  expect(createResponse.ok(), `${createBodyText}\nPayload: ${JSON.stringify(createPayload)}`).toBeTruthy();
  await waitForToast(page);
  await expect(page.getByRole("dialog", { name: /create invoice/i })).toHaveCount(0);

  const row = page.locator("[data-testid^='invoice-row-']").first();
  await expect(row).toBeVisible();

  await row.locator("[data-testid^='invoice-row-menu-']").click();
  await page.getByRole("menuitem", { name: /edit/i }).click();
  await page.getByTestId("invoice-form-status").selectOption("issued");
  await page.getByTestId("invoice-form-submit").click();
  await waitForToast(page);
  await expect(page.getByRole("dialog", { name: /edit invoice/i })).toHaveCount(0);

  await row.locator("[data-testid^='invoice-row-menu-']").click();
  await page.getByRole("menuitem", { name: /record payment/i }).click();
  await page.locator("label:has-text('Amount ($)')").locator("..")
    .locator("input").first().fill("150");
  await page.getByRole("button", { name: /record payment/i }).last().click();
  await waitForToast(page);

  await expect(row).toContainText(/paid|issued|overdue/i);
});

import { expect, type Locator, type Page } from "@playwright/test";

function escapeRegExp(value: string): string {
  return value.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
}

export function uniqueName(prefix: string): string {
  const stamp = Date.now().toString(36);
  return `e2e-${prefix}-${stamp}`;
}

export async function gotoSection(page: Page, label: string): Promise<void> {
  const sidebar = page.locator("aside");
  const button = sidebar.getByRole("button", { name: new RegExp(`^${label}$`, "i") }).first();
  await expect(button).toBeVisible();
  await button.click();
}

export async function openRowMenuByText(page: Page, text: string): Promise<void> {
  const row = page.getByRole("row").filter({ hasText: text }).first();
  await expect(row).toBeVisible();
  await row.getByRole("button").last().click();
}

export async function confirmDeleteDialog(page: Page): Promise<void> {
  const dialog = page.getByRole("dialog").filter({ hasText: /delete|revoke|remove/i }).last();
  const visible = await dialog.isVisible({ timeout: 2000 }).catch(() => false);
  if (!visible) return;
  await dialog.getByRole("button", { name: /delete|confirm|yes|revoke|remove/i }).first().click();
}

export async function fillLabeledInput(page: Page, label: string, value: string): Promise<void> {
  const labelNode = page
    .locator("label")
    .filter({ hasText: new RegExp(`^${escapeRegExp(label)}$`, "i") })
    .first();
  const container = labelNode.locator("xpath=..");
  const input = container.locator("input,textarea").first();
  await expect(input).toBeVisible();
  await input.fill(value);
}

export async function selectByLabel(page: Page, label: string, value: string): Promise<void> {
  const labelNode = page
    .locator("label")
    .filter({ hasText: new RegExp(`^${escapeRegExp(label)}$`, "i") })
    .first();
  const container = labelNode.locator("xpath=..");
  const select = container.locator("select").first();
  await expect(select).toBeVisible();
  await select.selectOption({ value });
}

export async function waitForToast(page: Page): Promise<Locator> {
  const toast = page.locator("[data-sonner-toast]").last();
  await expect(toast).toBeVisible();
  return toast;
}

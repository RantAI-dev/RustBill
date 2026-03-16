import "@testing-library/jest-dom/vitest";
import { afterAll, afterEach, beforeAll } from "vitest";
import { setupServer } from "msw/node";
import { http, HttpResponse } from "msw";

export const handlers = [
  http.get("/api/products", () => {
    return HttpResponse.json([
      { id: "1", name: "Test Product", productType: "licensed" },
    ]);
  }),
  http.get("/api/analytics/overview", () => {
    return HttpResponse.json({
      totalRevenue: "$10,000",
      platformUsers: "500",
      activeLicenses: "100",
      customerCount: "50",
    });
  }),
];

export const server = setupServer(...handlers);

beforeAll(() => server.listen({ onUnhandledRequest: "bypass" }));
afterEach(() => server.resetHandlers());
afterAll(() => server.close());

import { describe, it, expect, vi, beforeEach } from "vitest";
import { server } from "../setup";
import { http, HttpResponse } from "msw";

vi.mock("sonner", () => ({
  toast: { error: vi.fn(), success: vi.fn() },
}));

// Must import after mock
import { toast } from "sonner";
import {
  createProduct,
  deleteProduct,
  getCheckout,
  generateKeypair,
} from "@/hooks/use-api";

beforeEach(() => {
  vi.clearAllMocks();
});

describe("use-api mutation helpers", () => {
  it("createProduct returns success on 200", async () => {
    const product = { id: "p1", name: "Widget", productType: "licensed" };
    server.use(
      http.post("/api/products", () => {
        return HttpResponse.json(product, { status: 200 });
      }),
    );

    const result = await createProduct({ name: "Widget", productType: "licensed" });
    expect(result).toEqual({ success: true, data: product });
  });

  it("createProduct returns error on 400", async () => {
    server.use(
      http.post("/api/products", () => {
        return HttpResponse.json({ error: "Invalid data" }, { status: 400 });
      }),
    );

    const result = await createProduct({ name: "" });
    expect(result).toEqual({
      success: false,
      error: "Invalid data",
      status: 400,
    });
  });

  it("deleteProduct calls correct URL", async () => {
    let calledUrl = "";
    server.use(
      http.delete("/api/products/:id", ({ request }) => {
        calledUrl = new URL(request.url).pathname;
        return HttpResponse.json({ deleted: true });
      }),
    );

    const result = await deleteProduct("abc-123");
    expect(calledUrl).toBe("/api/products/abc-123");
    expect(result.success).toBe(true);
  });

  it("mutation shows toast on error", async () => {
    server.use(
      http.post("/api/products", () => {
        return HttpResponse.json({ error: "Server error" }, { status: 500 });
      }),
    );

    await createProduct({ name: "fail" });
    expect(toast.error).toHaveBeenCalledWith("Server error");
  });

  it("getCheckout returns structured result", async () => {
    const checkoutData = { checkoutUrl: "https://pay.example.com/sess_1", provider: "stripe" };
    server.use(
      http.get("/api/billing/checkout", () => {
        return HttpResponse.json(checkoutData, { status: 200 });
      }),
    );

    const result = await getCheckout("inv-1", "stripe");
    expect(result).toEqual({ success: true, data: checkoutData });
  });

  it("generateKeypair preserves status code on 409", async () => {
    server.use(
      http.post("/api/licenses/keypair", () => {
        return HttpResponse.json(
          { error: "Keypair already exists" },
          { status: 409 },
        );
      }),
    );

    const result = await generateKeypair();
    expect(result.success).toBe(false);
    if (!result.success) {
      expect(result.status).toBe(409);
      expect(result.error).toBe("Keypair already exists");
    }
  });

  it("fetcher throws on non-200 via mutate path", async () => {
    server.use(
      http.post("/api/products", () => {
        return HttpResponse.json({ error: "Not found" }, { status: 404 });
      }),
    );

    const result = await createProduct({ name: "test" });
    expect(result.success).toBe(false);
    if (!result.success) {
      expect(result.status).toBe(404);
    }
  });

  it("mutation handles network error", async () => {
    server.use(
      http.post("/api/products", () => {
        return HttpResponse.error();
      }),
    );

    const result = await createProduct({ name: "test" });
    expect(result.success).toBe(false);
    if (!result.success) {
      expect(result.error).toBe("Failed to create product");
      expect(result.status).toBeUndefined();
    }
    expect(toast.error).toHaveBeenCalledWith("Failed to create product");
  });

  it("mutation uses fallback error when response has no error field", async () => {
    server.use(
      http.post("/api/products", () => {
        return HttpResponse.json({}, { status: 422 });
      }),
    );

    const result = await createProduct({ name: "x" });
    expect(result.success).toBe(false);
    if (!result.success) {
      expect(result.error).toBe("Failed to create product");
      expect(result.status).toBe(422);
    }
  });

  it("mutation toast shows error from response body", async () => {
    server.use(
      http.post("/api/products", () => {
        return HttpResponse.json(
          { error: "Duplicate product name" },
          { status: 409 },
        );
      }),
    );

    await createProduct({ name: "dup" });
    expect(toast.error).toHaveBeenCalledWith("Duplicate product name");
  });
});

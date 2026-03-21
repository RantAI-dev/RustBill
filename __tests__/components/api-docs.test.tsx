import { beforeEach, describe, expect, it, vi } from "vitest";
import { fireEvent, render, screen, waitFor, within } from "@testing-library/react";

import { ApiDocsSection } from "@/components/dashboard/sections/api-docs";

describe("ApiDocsSection playground", () => {
  beforeEach(() => {
    vi.restoreAllMocks();
  });

  it("shows API key validation for public endpoints", async () => {
    render(<ApiDocsSection />);

    const playgroundTab = screen.getByRole("tab", { name: /playground/i });
    fireEvent.mouseDown(playgroundTab);
    fireEvent.click(playgroundTab);
    await screen.findByText(/API Playground/i);
    const panel = screen.getByRole("tabpanel", { name: /playground/i });
    fireEvent.click(within(panel).getByRole("button", { name: /send request/i }));

    expect(await screen.findByText(/requires an API key/i)).toBeInTheDocument();
  });

  it("sends admin request with session credentials", async () => {
    const fetchMock = vi
      .spyOn(globalThis, "fetch")
      .mockResolvedValue(new Response(JSON.stringify({ ok: true }), { status: 200 }));

    render(<ApiDocsSection />);

    const playgroundTab = screen.getByRole("tab", { name: /playground/i });
    fireEvent.mouseDown(playgroundTab);
    fireEvent.click(playgroundTab);
    await screen.findByText(/API Playground/i);
    const panel = screen.getByRole("tabpanel", { name: /playground/i });

    fireEvent.change(within(panel).getByRole("combobox", { name: "Scope" }), {
      target: { value: "admin" },
    });

    fireEvent.change(within(panel).getByRole("combobox", { name: "Endpoint" }), {
      target: { value: "admin-products-list" },
    });

    fireEvent.click(within(panel).getByRole("button", { name: /send request/i }));

    await waitFor(() => expect(fetchMock).toHaveBeenCalledTimes(1));
    expect(fetchMock).toHaveBeenCalledWith(
      "/api/products",
      expect.objectContaining({ method: "GET", credentials: "include" }),
    );
  });

  it("blocks invalid JSON body before request", async () => {
    const fetchMock = vi.spyOn(globalThis, "fetch");
    render(<ApiDocsSection />);

    const playgroundTab = screen.getByRole("tab", { name: /playground/i });
    fireEvent.mouseDown(playgroundTab);
    fireEvent.click(playgroundTab);
    await screen.findByText(/API Playground/i);
    const panel = screen.getByRole("tabpanel", { name: /playground/i });

    fireEvent.change(within(panel).getByLabelText(/api key/i), {
      target: { value: "pk_live_test" },
    });

    fireEvent.change(within(panel).getByRole("combobox", { name: "Endpoint" }), {
      target: { value: "v1-licenses-verify" },
    });

    fireEvent.change(within(panel).getByLabelText(/json body/i), {
      target: { value: "{ invalid" },
    });

    fireEvent.click(within(panel).getByRole("button", { name: /send request/i }));

    expect(await screen.findByText(/must be valid JSON/i)).toBeInTheDocument();
    expect(fetchMock).not.toHaveBeenCalled();
  });
});

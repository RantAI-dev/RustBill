import { describe, it, expect, vi } from "vitest";
import { render, screen, fireEvent } from "@testing-library/react";
import { ErrorBoundary } from "@/components/error-boundary";

function ThrowError(): never {
  throw new Error("Test error");
}

describe("ErrorBoundary", () => {
  it("renders children normally when no error", () => {
    render(
      <ErrorBoundary>
        <p>Hello world</p>
      </ErrorBoundary>,
    );
    expect(screen.getByText("Hello world")).toBeInTheDocument();
  });

  it("catches error and shows 'Something went wrong'", () => {
    const consoleSpy = vi.spyOn(console, "error").mockImplementation(() => {});

    render(
      <ErrorBoundary>
        <ThrowError />
      </ErrorBoundary>,
    );

    expect(screen.getByText("Something went wrong")).toBeInTheDocument();
    expect(
      screen.getByText(/An unexpected error occurred/),
    ).toBeInTheDocument();

    consoleSpy.mockRestore();
  });

  it("reload button calls window.location.reload", () => {
    const consoleSpy = vi.spyOn(console, "error").mockImplementation(() => {});
    const reloadMock = vi.fn();
    Object.defineProperty(window, "location", {
      value: { ...window.location, reload: reloadMock },
      writable: true,
    });

    render(
      <ErrorBoundary>
        <ThrowError />
      </ErrorBoundary>,
    );

    fireEvent.click(screen.getByRole("button", { name: /reload page/i }));
    expect(reloadMock).toHaveBeenCalled();

    consoleSpy.mockRestore();
  });
});

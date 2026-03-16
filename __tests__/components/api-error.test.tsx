import { describe, it, expect, vi } from "vitest";
import { render, screen, fireEvent } from "@testing-library/react";
import { ApiError } from "@/components/api-error";

describe("ApiError", () => {
  it("renders default message 'Something went wrong'", () => {
    render(<ApiError />);
    expect(screen.getByText("Something went wrong")).toBeInTheDocument();
  });

  it("renders custom message", () => {
    render(<ApiError message="Failed to load products" />);
    expect(screen.getByText("Failed to load products")).toBeInTheDocument();
    expect(screen.queryByText("Something went wrong")).not.toBeInTheDocument();
  });

  it("retry button calls onRetry callback", () => {
    const onRetry = vi.fn();
    render(<ApiError message="Error" onRetry={onRetry} />);

    const button = screen.getByRole("button", { name: /try again/i });
    fireEvent.click(button);
    expect(onRetry).toHaveBeenCalledOnce();
  });
});

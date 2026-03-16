import { describe, it, expect } from "vitest";
import { render, screen, fireEvent, act } from "@testing-library/react";
import {
  BackendStatusProvider,
  BackendBanner,
  useBackendStatus,
} from "@/components/backend-banner";

function TestWrapper({ children }: { children: React.ReactNode }) {
  return <BackendStatusProvider>{children}</BackendStatusProvider>;
}

function TriggerDown() {
  const { setBackendDown } = useBackendStatus();
  return <button onClick={() => setBackendDown(true)}>trigger</button>;
}

function TriggerUp() {
  const { clearBackendDown } = useBackendStatus();
  return <button onClick={() => clearBackendDown()}>recover</button>;
}

describe("BackendBanner", () => {
  it("banner hidden when backendDown is false", () => {
    render(
      <TestWrapper>
        <BackendBanner />
      </TestWrapper>,
    );
    expect(screen.queryByText(/unavailable/)).not.toBeInTheDocument();
  });

  it("banner visible when backendDown is true", () => {
    render(
      <TestWrapper>
        <TriggerDown />
        <BackendBanner />
      </TestWrapper>,
    );

    act(() => {
      fireEvent.click(screen.getByText("trigger"));
    });

    expect(screen.getByText(/unavailable/)).toBeInTheDocument();
  });

  it("dismiss hides the banner", () => {
    render(
      <TestWrapper>
        <TriggerDown />
        <BackendBanner />
      </TestWrapper>,
    );

    act(() => {
      fireEvent.click(screen.getByText("trigger"));
    });

    expect(screen.getByText(/unavailable/)).toBeInTheDocument();

    // Click the X / dismiss button
    const dismissButton = screen.getByRole("button", { name: "" });
    act(() => {
      fireEvent.click(dismissButton);
    });

    expect(screen.queryByText(/unavailable/)).not.toBeInTheDocument();
  });

  it("banner reappears after recovery then failure", () => {
    render(
      <TestWrapper>
        <TriggerDown />
        <TriggerUp />
        <BackendBanner />
      </TestWrapper>,
    );

    // First failure
    act(() => {
      fireEvent.click(screen.getByText("trigger"));
    });
    expect(screen.getByText(/unavailable/)).toBeInTheDocument();

    // Dismiss
    const dismissButton = screen.getByRole("button", { name: "" });
    act(() => {
      fireEvent.click(dismissButton);
    });
    expect(screen.queryByText(/unavailable/)).not.toBeInTheDocument();

    // Recovery
    act(() => {
      fireEvent.click(screen.getByText("recover"));
    });

    // Second failure — banner should reappear (dismissed reset by useEffect)
    act(() => {
      fireEvent.click(screen.getByText("trigger"));
    });
    expect(screen.getByText(/unavailable/)).toBeInTheDocument();
  });
});

"use client";

import React, { createContext, useContext, useState, useCallback, useEffect } from "react";
import { WifiOff, X } from "lucide-react";

interface BackendStatusContextType {
  backendDown: boolean;
  setBackendDown: (down: boolean) => void;
  clearBackendDown: () => void;
}

const BackendStatusContext = createContext<BackendStatusContextType>({
  backendDown: false,
  setBackendDown: () => {},
  clearBackendDown: () => {},
});

export function useBackendStatus() {
  return useContext(BackendStatusContext);
}

export function BackendStatusProvider({ children }: { children: React.ReactNode }) {
  const [backendDown, setBackendDown] = useState(false);
  const clearBackendDown = useCallback(() => setBackendDown(false), []);

  return (
    <BackendStatusContext.Provider value={{ backendDown, setBackendDown, clearBackendDown }}>
      {children}
    </BackendStatusContext.Provider>
  );
}

export function BackendBanner() {
  const { backendDown } = useBackendStatus();
  const [dismissed, setDismissed] = useState(false);

  // Reset dismissed state when backend goes down again after recovery
  useEffect(() => {
    if (backendDown) setDismissed(false);
  }, [backendDown]);

  if (!backendDown || dismissed) return null;

  return (
    <div className="bg-destructive/10 border-b border-destructive/20 px-4 py-2 flex items-center justify-between">
      <div className="flex items-center gap-2 text-sm text-destructive">
        <WifiOff className="h-4 w-4" />
        <span>Backend service is unavailable. Some features may not work.</span>
      </div>
      <button
        onClick={() => setDismissed(true)}
        className="text-destructive/60 hover:text-destructive"
      >
        <X className="h-4 w-4" />
      </button>
    </div>
  );
}

"use client";

import { AlertTriangle } from "lucide-react";
import { Button } from "@/components/ui/button";

interface ApiErrorProps {
  message?: string;
  onRetry?: () => void;
}

export function ApiError({ message = "Something went wrong", onRetry }: ApiErrorProps) {
  return (
    <div className="flex flex-col items-center justify-center py-12 text-center space-y-3">
      <AlertTriangle className="h-8 w-8 text-destructive" />
      <p className="text-sm text-muted-foreground">{message}</p>
      {onRetry && (
        <Button variant="outline" size="sm" onClick={onRetry}>
          Try Again
        </Button>
      )}
    </div>
  );
}

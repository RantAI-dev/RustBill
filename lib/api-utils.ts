import { NextResponse } from "next/server";

/**
 * Handle unexpected API errors — log full details, return generic message.
 */
export function handleApiError(error: unknown, context?: string): NextResponse {
  console.error(`API Error${context ? ` [${context}]` : ""}:`, error);
  return NextResponse.json(
    { error: "An internal error occurred. Please try again later." },
    { status: 500 },
  );
}

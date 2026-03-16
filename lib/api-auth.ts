import { db } from "@/lib/db";
import { apiKeys } from "@/lib/db/schema";
import { eq, and } from "drizzle-orm";
import { hashApiKey } from "@/lib/api-keys";
import { checkRateLimit } from "@/lib/rate-limit";
import { NextResponse } from "next/server";

type AuthResult =
  | { success: true; apiKey: typeof apiKeys.$inferSelect }
  | { success: false; response: NextResponse };

export async function authenticateApiKey(request: Request): Promise<AuthResult> {
  // 1. Extract Bearer token
  const authHeader = request.headers.get("authorization");
  if (!authHeader?.startsWith("Bearer ")) {
    return {
      success: false,
      response: NextResponse.json(
        { error: "missing_api_key", message: "Authorization header with Bearer token required" },
        { status: 401 },
      ),
    };
  }

  const apiKey = authHeader.slice(7);
  const keyHash = hashApiKey(apiKey);

  // 2. Look up key by hash, must be active
  const [keyRecord] = await db
    .select()
    .from(apiKeys)
    .where(and(eq(apiKeys.keyHash, keyHash), eq(apiKeys.status, "active")));

  if (!keyRecord) {
    return {
      success: false,
      response: NextResponse.json(
        { error: "invalid_api_key", message: "API key is invalid or revoked" },
        { status: 401 },
      ),
    };
  }

  // 3. Rate limit check
  const rateLimitResult = checkRateLimit(keyRecord.id);
  if (!rateLimitResult.allowed) {
    return {
      success: false,
      response: NextResponse.json(
        { error: "rate_limited", message: "Too many requests", retryAfter: rateLimitResult.retryAfter },
        { status: 429, headers: { "Retry-After": String(rateLimitResult.retryAfter) } },
      ),
    };
  }

  // 4. Update lastUsedAt (fire-and-forget)
  db.update(apiKeys)
    .set({ lastUsedAt: new Date() })
    .where(eq(apiKeys.id, keyRecord.id))
    .then(() => {});

  return { success: true, apiKey: keyRecord };
}

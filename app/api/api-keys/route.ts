import { db } from "@/lib/db";
import { apiKeys } from "@/lib/db/schema";
import { createApiKeySchema } from "@/lib/validations/api-keys";
import { generateApiKey, hashApiKey, getKeyPrefix } from "@/lib/api-keys";
import { desc } from "drizzle-orm";
import { NextRequest, NextResponse } from "next/server";

export async function GET() {
  const rows = await db
    .select({
      id: apiKeys.id,
      name: apiKeys.name,
      keyPrefix: apiKeys.keyPrefix,
      status: apiKeys.status,
      lastUsedAt: apiKeys.lastUsedAt,
      createdAt: apiKeys.createdAt,
    })
    .from(apiKeys)
    .orderBy(desc(apiKeys.createdAt));

  return NextResponse.json(rows);
}

export async function POST(req: NextRequest) {
  const body = await req.json();
  const parsed = createApiKeySchema.safeParse(body);
  if (!parsed.success) {
    return NextResponse.json({ error: parsed.error.flatten() }, { status: 400 });
  }

  const key = generateApiKey();
  const keyHash = hashApiKey(key);
  const keyPrefix = getKeyPrefix(key);

  const [row] = await db
    .insert(apiKeys)
    .values({
      name: parsed.data.name,
      keyHash,
      keyPrefix,
    })
    .returning();

  return NextResponse.json(
    {
      id: row.id,
      name: row.name,
      key, // Full key — shown only once
      keyPrefix: row.keyPrefix,
      status: row.status,
      createdAt: row.createdAt,
    },
    { status: 201 },
  );
}

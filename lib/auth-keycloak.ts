import { db } from "@/lib/db";
import { users } from "@/lib/db/schema";
import { eq } from "drizzle-orm";
import { getKeycloakConfig } from "@/lib/auth-config";

export interface KeycloakProfile {
  sub: string;
  email?: string;
  name?: string;
  preferred_username?: string;
  realm_access?: { roles?: string[] };
}

/**
 * Decode JWT payload from a base64url-encoded token.
 * Safe for tokens received from a server-to-server exchange (not browser).
 */
export function decodeJwtPayload(token: string): Record<string, unknown> {
  const payload = token.split(".")[1];
  if (!payload) throw new Error("Invalid JWT: no payload segment");
  return JSON.parse(Buffer.from(payload, "base64url").toString());
}

/**
 * Find an existing user by email or create a new one from Keycloak profile.
 * Syncs role and name from Keycloak on every login.
 */
export async function findOrCreateKeycloakUser(profile: KeycloakProfile) {
  const config = getKeycloakConfig();

  const keycloakRoles = profile.realm_access?.roles ?? [];
  let localRole: "admin" | "customer" = "customer";
  if (keycloakRoles.includes(config.adminRole)) {
    localRole = "admin";
  }

  const email = profile.email!.toLowerCase();
  const displayName = profile.name || profile.preferred_username || email;

  // Try to find existing user by email
  const [existing] = await db
    .select()
    .from(users)
    .where(eq(users.email, email))
    .limit(1);

  if (existing) {
    // Sync role and name from Keycloak
    await db
      .update(users)
      .set({
        name: displayName,
        role: localRole,
        authProvider: "keycloak",
        updatedAt: new Date(),
      })
      .where(eq(users.id, existing.id));

    return { ...existing, name: displayName, role: localRole, authProvider: "keycloak" as const };
  }

  // Create new user (no password hash for Keycloak users)
  const [newUser] = await db
    .insert(users)
    .values({
      email,
      name: displayName,
      passwordHash: null,
      role: localRole,
      authProvider: "keycloak",
    })
    .returning();

  return newUser;
}

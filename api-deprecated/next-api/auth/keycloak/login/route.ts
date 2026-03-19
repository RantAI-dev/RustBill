import crypto from "crypto";
import { cookies } from "next/headers";
import { NextRequest, NextResponse } from "next/server";
import { getAuthProvider, getKeycloakConfig } from "@/lib/auth-config";

/**
 * GET /api/auth/keycloak/login
 * Initiates OIDC Authorization Code flow — redirects browser to Keycloak.
 */
export async function GET(req: NextRequest) {
  if (getAuthProvider() !== "keycloak") {
    return NextResponse.json({ error: "Keycloak auth is not enabled" }, { status: 400 });
  }

  const config = getKeycloakConfig();
  const { searchParams } = new URL(req.url);
  const from = searchParams.get("from") || "/";

  // Generate CSRF state
  const state = crypto.randomBytes(32).toString("hex");

  // Store state + return path in cookies
  const cookieStore = await cookies();
  cookieStore.set("keycloak_state", state, {
    httpOnly: true,
    secure: process.env.NODE_ENV === "production",
    sameSite: "lax",
    path: "/",
    maxAge: 300, // 5 minutes
  });
  cookieStore.set("keycloak_from", from, {
    httpOnly: true,
    secure: process.env.NODE_ENV === "production",
    sameSite: "lax",
    path: "/",
    maxAge: 300,
  });

  const origin = req.headers.get("origin") ?? req.nextUrl.origin;
  const redirectUri = `${origin}/api/auth/keycloak/callback`;

  const authUrl = new URL(config.authorizationEndpoint);
  authUrl.searchParams.set("response_type", "code");
  authUrl.searchParams.set("client_id", config.clientId);
  authUrl.searchParams.set("redirect_uri", redirectUri);
  authUrl.searchParams.set("scope", "openid email profile");
  authUrl.searchParams.set("state", state);

  return NextResponse.redirect(authUrl.toString());
}

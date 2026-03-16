import { cookies } from "next/headers";
import { NextRequest, NextResponse } from "next/server";
import { getAuthProvider, getKeycloakConfig } from "@/lib/auth-config";
import { decodeJwtPayload, findOrCreateKeycloakUser } from "@/lib/auth-keycloak";
import type { KeycloakProfile } from "@/lib/auth-keycloak";
import { createSession, setSessionCookie } from "@/lib/auth";

/**
 * GET /api/auth/keycloak/callback
 * Handles OIDC callback — exchanges code for tokens, creates/finds user, establishes session.
 */
export async function GET(req: NextRequest) {
  if (getAuthProvider() !== "keycloak") {
    return NextResponse.json({ error: "Keycloak auth is not enabled" }, { status: 400 });
  }

  const config = getKeycloakConfig();
  const { searchParams } = new URL(req.url);
  const code = searchParams.get("code");
  const state = searchParams.get("state");
  const error = searchParams.get("error");

  // Handle Keycloak error response
  if (error) {
    const desc = searchParams.get("error_description") || error;
    return NextResponse.redirect(
      new URL(`/login?error=${encodeURIComponent(desc)}`, req.nextUrl.origin),
    );
  }

  if (!code || !state) {
    return NextResponse.redirect(new URL("/login?error=missing_params", req.nextUrl.origin));
  }

  // Validate CSRF state
  const cookieStore = await cookies();
  const savedState = cookieStore.get("keycloak_state")?.value;
  const from = cookieStore.get("keycloak_from")?.value || "/";

  // Clean up OIDC cookies
  cookieStore.delete("keycloak_state");
  cookieStore.delete("keycloak_from");

  if (!savedState || savedState !== state) {
    return NextResponse.redirect(new URL("/login?error=invalid_state", req.nextUrl.origin));
  }

  // Exchange authorization code for tokens
  const origin = req.headers.get("origin") ?? req.nextUrl.origin;
  const redirectUri = `${origin}/api/auth/keycloak/callback`;

  let tokenData: { id_token?: string; access_token?: string };
  try {
    const tokenRes = await fetch(config.tokenEndpoint, {
      method: "POST",
      headers: { "Content-Type": "application/x-www-form-urlencoded" },
      body: new URLSearchParams({
        grant_type: "authorization_code",
        code,
        redirect_uri: redirectUri,
        client_id: config.clientId,
        client_secret: config.clientSecret,
      }),
    });

    if (!tokenRes.ok) {
      const body = await tokenRes.text();
      console.error("Keycloak token exchange failed:", tokenRes.status, body);
      return NextResponse.redirect(new URL("/login?error=token_exchange_failed", req.nextUrl.origin));
    }

    tokenData = await tokenRes.json();
  } catch (err) {
    console.error("Keycloak token exchange error:", err);
    return NextResponse.redirect(new URL("/login?error=token_exchange_failed", req.nextUrl.origin));
  }

  // Decode id_token to get user profile
  if (!tokenData.id_token) {
    return NextResponse.redirect(new URL("/login?error=no_id_token", req.nextUrl.origin));
  }

  let profile: KeycloakProfile;
  try {
    profile = decodeJwtPayload(tokenData.id_token) as unknown as KeycloakProfile;
  } catch {
    return NextResponse.redirect(new URL("/login?error=invalid_token", req.nextUrl.origin));
  }

  if (!profile.email) {
    return NextResponse.redirect(
      new URL("/login?error=no_email", req.nextUrl.origin),
    );
  }

  // Find or create local user
  const user = await findOrCreateKeycloakUser(profile);

  // Admin-only policy (same as default login)
  if (user.role !== "admin") {
    return NextResponse.redirect(
      new URL("/login?error=admin_only", req.nextUrl.origin),
    );
  }

  // Create local DB session
  const token = await createSession(user.id);
  await setSessionCookie(token);

  return NextResponse.redirect(new URL(from, req.nextUrl.origin));
}

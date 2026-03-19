import { NextResponse } from "next/server";
import { getSessionToken, deleteSession, clearSessionCookie } from "@/lib/auth";
import { getAuthProvider, getKeycloakConfig } from "@/lib/auth-config";

export async function POST() {
  try {
    const token = await getSessionToken();

    if (token) {
      await deleteSession(token);
    }

    await clearSessionCookie();

    // For Keycloak, return the end-session URL so the client can redirect
    if (getAuthProvider() === "keycloak") {
      try {
        const config = getKeycloakConfig();
        const appUrl = process.env.NEXT_PUBLIC_APP_URL || "http://localhost:3000";
        const postLogoutRedirect = `${appUrl}/login`;
        const logoutUrl = `${config.endSessionEndpoint}?post_logout_redirect_uri=${encodeURIComponent(postLogoutRedirect)}&client_id=${config.clientId}`;
        return NextResponse.json({ ok: true, redirectUrl: logoutUrl });
      } catch {
        // Keycloak not configured — fall through to default logout
      }
    }

    return NextResponse.json({ ok: true });
  } catch {
    return NextResponse.json(
      { error: "Internal server error" },
      { status: 500 }
    );
  }
}

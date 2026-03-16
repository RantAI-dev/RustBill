export type AuthProvider = "default" | "keycloak";

export function getAuthProvider(): AuthProvider {
  const provider = process.env.AUTH_PROVIDER;
  if (provider === "keycloak") return "keycloak";
  return "default";
}

export function getKeycloakConfig() {
  const issuerUrl = process.env.KEYCLOAK_ISSUER_URL;
  const clientId = process.env.KEYCLOAK_CLIENT_ID;
  const clientSecret = process.env.KEYCLOAK_CLIENT_SECRET;

  if (!issuerUrl || !clientId || !clientSecret) {
    throw new Error(
      "Missing Keycloak configuration. Set KEYCLOAK_ISSUER_URL, KEYCLOAK_CLIENT_ID, and KEYCLOAK_CLIENT_SECRET.",
    );
  }

  return {
    issuerUrl,
    clientId,
    clientSecret,
    adminRole: process.env.KEYCLOAK_ADMIN_ROLE || "admin",
    customerRole: process.env.KEYCLOAK_CUSTOMER_ROLE || "customer",
    // Standard Keycloak OIDC endpoints
    authorizationEndpoint: `${issuerUrl}/protocol/openid-connect/auth`,
    tokenEndpoint: `${issuerUrl}/protocol/openid-connect/token`,
    userinfoEndpoint: `${issuerUrl}/protocol/openid-connect/userinfo`,
    endSessionEndpoint: `${issuerUrl}/protocol/openid-connect/logout`,
  };
}

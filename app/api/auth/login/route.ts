import { NextResponse } from "next/server";
import { eq } from "drizzle-orm";
import { db } from "@/lib/db";
import { users } from "@/lib/db/schema";
import { verifyPassword, createSession, setSessionCookie, deleteSession } from "@/lib/auth";
import { loginSchema } from "@/lib/validations/auth";
import { getAuthProvider } from "@/lib/auth-config";

export async function POST(req: Request) {
  try {
    // Block email/password login when Keycloak is active
    if (getAuthProvider() === "keycloak") {
      return NextResponse.json(
        { error: "Email/password login is disabled. Use SSO." },
        { status: 400 },
      );
    }

    const body = await req.json();
    const parsed = loginSchema.safeParse(body);

    if (!parsed.success) {
      return NextResponse.json(
        { error: "Invalid email or password" },
        { status: 400 }
      );
    }

    const { email, password } = parsed.data;

    // Find user by email
    const [user] = await db
      .select()
      .from(users)
      .where(eq(users.email, email.toLowerCase()))
      .limit(1);

    if (!user) {
      return NextResponse.json(
        { error: "Invalid email or password" },
        { status: 401 }
      );
    }

    // Reject Keycloak-provisioned users without a local password
    if (!user.passwordHash) {
      return NextResponse.json(
        { error: "Invalid email or password" },
        { status: 401 },
      );
    }

    // Verify password
    const valid = await verifyPassword(password, user.passwordHash);
    if (!valid) {
      return NextResponse.json(
        { error: "Invalid email or password" },
        { status: 401 }
      );
    }

    // Create session
    const token = await createSession(user.id);

    // Non-admin users: destroy session and block
    if (user.role !== "admin") {
      await deleteSession(token);
      return NextResponse.json(
        { error: "Access restricted to administrators" },
        { status: 403 }
      );
    }

    // Set cookie
    await setSessionCookie(token);

    return NextResponse.json({
      user: {
        id: user.id,
        name: user.name,
        email: user.email,
        role: user.role,
      },
    });
  } catch {
    return NextResponse.json(
      { error: "Internal server error" },
      { status: 500 }
    );
  }
}

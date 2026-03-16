import { randomBytes } from "crypto";
import bcrypt from "bcryptjs";
import { eq } from "drizzle-orm";
import { cookies } from "next/headers";
import { NextResponse } from "next/server";
import { db } from "./db";
import { users, sessions } from "./db/schema";

const SESSION_COOKIE = "session";
const SESSION_EXPIRY_DAYS = 7;

// ---- Password helpers ----

export async function hashPassword(password: string): Promise<string> {
  return bcrypt.hash(password, 12);
}

export async function verifyPassword(password: string, hash: string): Promise<boolean> {
  return bcrypt.compare(password, hash);
}

// ---- Session helpers ----

function generateToken(): string {
  return randomBytes(32).toString("hex"); // 64 chars
}

export async function createSession(userId: string): Promise<string> {
  const token = generateToken();
  const expiresAt = new Date(Date.now() + SESSION_EXPIRY_DAYS * 24 * 60 * 60 * 1000);

  await db.insert(sessions).values({
    id: token,
    userId,
    expiresAt,
  });

  return token;
}

export async function validateSession(token: string) {
  const [session] = await db
    .select({
      sessionId: sessions.id,
      userId: sessions.userId,
      expiresAt: sessions.expiresAt,
      userName: users.name,
      userEmail: users.email,
      userRole: users.role,
      userCustomerId: users.customerId,
    })
    .from(sessions)
    .innerJoin(users, eq(sessions.userId, users.id))
    .where(eq(sessions.id, token))
    .limit(1);

  if (!session || session.expiresAt < new Date()) {
    return null;
  }

  return {
    id: session.userId,
    name: session.userName,
    email: session.userEmail,
    role: session.userRole,
    customerId: session.userCustomerId,
  };
}

export async function deleteSession(token: string) {
  await db.delete(sessions).where(eq(sessions.id, token));
}

// ---- Cookie helpers ----

export async function setSessionCookie(token: string) {
  const cookieStore = await cookies();
  const isHttps = process.env.NEXTAUTH_URL?.startsWith("https://") ?? false;
  cookieStore.set(SESSION_COOKIE, token, {
    httpOnly: true,
    secure: isHttps,
    sameSite: "lax",
    path: "/",
    maxAge: SESSION_EXPIRY_DAYS * 24 * 60 * 60,
  });
}

export async function clearSessionCookie() {
  const cookieStore = await cookies();
  cookieStore.delete(SESSION_COOKIE);
}

export async function getSessionToken(): Promise<string | undefined> {
  const cookieStore = await cookies();
  return cookieStore.get(SESSION_COOKIE)?.value;
}

// ---- Auth helpers for API routes / server components ----

export async function getCurrentUser() {
  const token = await getSessionToken();
  if (!token) return null;
  return validateSession(token);
}

export async function requireAuth() {
  const user = await getCurrentUser();
  if (!user) {
    throw new Error("Unauthorized");
  }
  return user;
}

export async function requireAdmin() {
  const user = await requireAuth();
  if (user.role !== "admin") {
    throw new Error("Forbidden");
  }
  return user;
}

// ---- Non-throwing auth helpers for API route handlers ----

export type AuthUser = {
  id: string;
  name: string;
  email: string;
  role: string;
  customerId: string | null;
};

export type AuthResult =
  | { success: true; user: AuthUser }
  | { success: false; response: NextResponse };

export async function withAuth(): Promise<AuthResult> {
  const user = await getCurrentUser();
  if (!user) {
    return { success: false, response: NextResponse.json({ error: "Unauthorized" }, { status: 401 }) };
  }
  return { success: true, user };
}

export async function withAdmin(): Promise<AuthResult> {
  const result = await withAuth();
  if (!result.success) return result;
  if (result.user.role !== "admin") {
    return { success: false, response: NextResponse.json({ error: "Forbidden" }, { status: 403 }) };
  }
  return result;
}

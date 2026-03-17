"use client";

import { Suspense, useState } from "react";
import { useRouter, useSearchParams } from "next/navigation";
import { Loader2, LogIn, Shield } from "lucide-react";
import Image from "next/image";
import { appConfig } from "@/lib/app-config";

const authProvider = process.env.NEXT_PUBLIC_AUTH_PROVIDER || "default";

function LoginForm() {
  const router = useRouter();
  const searchParams = useSearchParams();
  const from = searchParams.get("from") || "/";

  const [email, setEmail] = useState("");
  const [password, setPassword] = useState("");
  const [error, setError] = useState("");
  const [loading, setLoading] = useState(false);

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    setError("");
    setLoading(true);

    try {
      const res = await fetch("/api/auth/login", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ email, password }),
      });

      const data = await res.json();

      if (!res.ok) {
        setError(data.error || "Login failed");
        return;
      }

      router.push(from);
    } catch {
      setError("Something went wrong. Please try again.");
    } finally {
      setLoading(false);
    }
  };

  return (
    <div className="bg-card border border-border rounded-xl p-6 shadow-lg animate-in fade-in slide-in-from-bottom-4 duration-500">
      <form onSubmit={handleSubmit} className="space-y-4">
        {error && (
          <div className="px-3 py-2 rounded-lg bg-destructive/10 border border-destructive/20 text-sm text-destructive">
            {error}
          </div>
        )}

        <div className="space-y-2">
          <label htmlFor="email" className="text-sm font-medium text-foreground">
            Email
          </label>
          <input
            id="email"
            type="email"
            value={email}
            onChange={(e) => setEmail(e.target.value)}
            placeholder="admin@rantai.com"
            required
            autoFocus
            className="w-full h-10 px-3 rounded-lg bg-secondary border border-border text-sm text-foreground placeholder:text-muted-foreground focus:outline-none focus:ring-2 focus:ring-ring/20 focus:border-accent transition-all duration-200"
          />
        </div>

        <div className="space-y-2">
          <label htmlFor="password" className="text-sm font-medium text-foreground">
            Password
          </label>
          <input
            id="password"
            type="password"
            value={password}
            onChange={(e) => setPassword(e.target.value)}
            placeholder="Enter your password"
            required
            className="w-full h-10 px-3 rounded-lg bg-secondary border border-border text-sm text-foreground placeholder:text-muted-foreground focus:outline-none focus:ring-2 focus:ring-ring/20 focus:border-accent transition-all duration-200"
          />
        </div>

        <button
          type="submit"
          disabled={loading}
          className="w-full h-10 rounded-lg bg-accent text-accent-foreground text-sm font-medium flex items-center justify-center gap-2 hover:bg-accent/90 disabled:opacity-50 disabled:cursor-not-allowed transition-all duration-200"
        >
          {loading ? (
            <Loader2 className="w-4 h-4 animate-spin" />
          ) : (
            <>
              <LogIn className="w-4 h-4" />
              Sign in
            </>
          )}
        </button>
      </form>
    </div>
  );
}

function KeycloakLogin() {
  const searchParams = useSearchParams();
  const from = searchParams.get("from") || "/";
  const errorParam = searchParams.get("error");
  const [loading, setLoading] = useState(false);

  const errorMessages: Record<string, string> = {
    no_email: "Your Keycloak account has no email address. Contact your administrator.",
    admin_only: "Access restricted to administrators.",
    invalid_state: "Login session expired. Please try again.",
    token_exchange_failed: "Authentication failed. Please try again.",
    missing_params: "Invalid callback. Please try again.",
    no_id_token: "Authentication failed. Please try again.",
    invalid_token: "Authentication failed. Please try again.",
  };

  const handleLogin = () => {
    setLoading(true);
    window.location.href = `/api/auth/keycloak/login?from=${encodeURIComponent(from)}`;
  };

  return (
    <div className="bg-card border border-border rounded-xl p-6 shadow-lg animate-in fade-in slide-in-from-bottom-4 duration-500">
      <div className="space-y-4">
        {errorParam && (
          <div className="px-3 py-2 rounded-lg bg-destructive/10 border border-destructive/20 text-sm text-destructive">
            {errorMessages[errorParam] || decodeURIComponent(errorParam)}
          </div>
        )}

        <button
          onClick={handleLogin}
          disabled={loading}
          className="w-full h-10 rounded-lg bg-accent text-accent-foreground text-sm font-medium flex items-center justify-center gap-2 hover:bg-accent/90 disabled:opacity-50 disabled:cursor-not-allowed transition-all duration-200"
        >
          {loading ? (
            <Loader2 className="w-4 h-4 animate-spin" />
          ) : (
            <>
              <Shield className="w-4 h-4" />
              Sign in with SSO
            </>
          )}
        </button>
      </div>
    </div>
  );
}

export default function LoginPage() {
  return (
    <div className="min-h-screen bg-background flex items-center justify-center p-4">
      <div className="w-full max-w-sm">
        {/* Branding */}
        <div className="text-center mb-8">
          <div className="inline-flex items-center justify-center mb-2">
            <Image
              src={appConfig.logoFull}
              alt={appConfig.name}
              width={280}
              height={80}
              className="h-16 w-auto object-contain"
              priority
            />
          </div>
          <p className="text-sm text-muted-foreground">Sign in to your dashboard</p>
        </div>

        <Suspense>
          {authProvider === "keycloak" ? <KeycloakLogin /> : <LoginForm />}
        </Suspense>

        <p className="text-center text-xs text-muted-foreground mt-4">
          {appConfig.name} &middot; Admin Access Only
        </p>
      </div>
    </div>
  );
}

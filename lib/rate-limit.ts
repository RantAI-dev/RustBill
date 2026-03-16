const LIMITS = {
  global: { windowMs: 60_000, max: 60 },
  customer: { windowMs: 60_000, max: 30 },
  webhook: { windowMs: 60_000, max: 10 },
} as const;

export type RateLimitTier = keyof typeof LIMITS;

const store = new Map<string, number[]>();

// Clean up stale entries every 5 minutes
if (typeof setInterval !== "undefined") {
  setInterval(() => {
    const now = Date.now();
    for (const [key, timestamps] of store) {
      const maxWindow = Math.max(...Object.values(LIMITS).map((l) => l.windowMs));
      const filtered = timestamps.filter((t) => now - t < maxWindow);
      if (filtered.length === 0) store.delete(key);
      else store.set(key, filtered);
    }
  }, 300_000);
}

export function checkRateLimit(
  identifier: string,
  tier: RateLimitTier = "global",
): { allowed: boolean; retryAfter: number } {
  const { windowMs, max } = LIMITS[tier];
  const now = Date.now();
  let timestamps = store.get(identifier);

  if (!timestamps) {
    timestamps = [];
    store.set(identifier, timestamps);
  }

  // Remove timestamps outside the window
  const filtered = timestamps.filter((t) => now - t < windowMs);
  store.set(identifier, filtered);

  if (filtered.length >= max) {
    const oldest = filtered[0];
    const retryAfter = Math.ceil((oldest + windowMs - now) / 1000);
    return { allowed: false, retryAfter };
  }

  filtered.push(now);
  return { allowed: true, retryAfter: 0 };
}

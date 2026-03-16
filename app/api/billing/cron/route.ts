import { NextRequest, NextResponse } from "next/server";
import { verifyCronSecret } from "@/lib/billing/cron-auth";
import { withAdmin } from "@/lib/auth";
import { handleApiError } from "@/lib/api-utils";

// Cron trigger -- calls the lifecycle endpoint to process subscriptions.
// Can be triggered by Vercel Cron, external scheduler, or manual call.
export async function POST(req: NextRequest) {
  try {
    const cronCheck = verifyCronSecret(req);
    if (cronCheck) {
      const auth = await withAdmin();
      if (!auth.success) return cronCheck;
    }

    const origin = new URL(req.url).origin;

    const res = await fetch(`${origin}/api/billing/subscriptions/lifecycle`, {
      method: "POST",
      headers: {
        cookie: req.headers.get("cookie") ?? "",
        "x-cron-secret": req.headers.get("x-cron-secret") ?? "",
        authorization: req.headers.get("authorization") ?? "",
      },
    });

    const data = await res.json();
    return NextResponse.json({ triggered: true, ...data });
  } catch (error) {
    return handleApiError(error, "POST /api/billing/cron");
  }
}

// GET variant for simple cron services that only support GET
export async function GET(req: NextRequest) {
  try {
    const cronCheck = verifyCronSecret(req);
    if (cronCheck) {
      const auth = await withAdmin();
      if (!auth.success) return cronCheck;
    }

    const origin = new URL(req.url).origin;

    const res = await fetch(`${origin}/api/billing/subscriptions/lifecycle`, {
      method: "POST",
      headers: {
        cookie: req.headers.get("cookie") ?? "",
        "x-cron-secret": req.headers.get("x-cron-secret") ?? "",
        authorization: req.headers.get("authorization") ?? "",
      },
    });

    const data = await res.json();
    return NextResponse.json({ triggered: true, ...data });
  } catch (error) {
    return handleApiError(error, "GET /api/billing/cron");
  }
}

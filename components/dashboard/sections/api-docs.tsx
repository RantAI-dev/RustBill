"use client";

import { useState } from "react";
import { cn } from "@/lib/utils";
import { appConfig } from "@/lib/app-config";
import { Copy, Check, Key, ShieldCheck, Zap, Send, Package, Building2, Handshake, KeyRound, CreditCard, FileText, Activity } from "lucide-react";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { Tabs, TabsContent, TabsList, TabsTrigger } from "@/components/ui/tabs";

/* ---------- copy helper ---------- */

function CopyBlock({ code, label }: { code: string; label?: string }) {
  const [copied, setCopied] = useState(false);

  const copy = () => {
    navigator.clipboard.writeText(code);
    setCopied(true);
    setTimeout(() => setCopied(false), 2000);
  };

  return (
    <div className="relative group">
      {label && <p className="text-xs text-muted-foreground mb-1.5">{label}</p>}
      <div className="bg-secondary rounded-lg border border-border overflow-hidden">
        <pre className="p-4 text-xs font-mono text-foreground overflow-x-auto whitespace-pre">{code}</pre>
        <button
          onClick={copy}
          className="absolute top-2 right-2 p-1.5 rounded-md bg-card border border-border text-muted-foreground hover:text-foreground opacity-0 group-hover:opacity-100 transition-all"
        >
          {copied ? <Check className="w-3.5 h-3.5 text-accent" /> : <Copy className="w-3.5 h-3.5" />}
        </button>
      </div>
    </div>
  );
}

/* ---------- method badge ---------- */

const methodColors: Record<string, string> = {
  GET: "bg-blue-500/10 text-blue-500 border-blue-500/20",
  POST: "bg-sky-500/10 text-sky-500 border-sky-500/20",
  PUT: "bg-amber-500/10 text-amber-500 border-amber-500/20",
  DELETE: "bg-red-500/10 text-red-500 border-red-500/20",
};

function MethodBadge({ method }: { method: string }) {
  return (
    <span className={cn("inline-flex items-center px-2 py-0.5 rounded text-[11px] font-semibold font-mono border", methodColors[method])}>
      {method}
    </span>
  );
}

/* ---------- endpoint row ---------- */

function EndpointRow({ method, path, description }: { method: string; path: string; description: string }) {
  return (
    <div className="flex items-start gap-3 py-2.5 border-b border-border last:border-0">
      <MethodBadge method={method} />
      <code className="text-sm font-mono text-foreground shrink-0">{path}</code>
      <span className="text-sm text-muted-foreground ml-auto text-right">{description}</span>
    </div>
  );
}

/* ---------- main section ---------- */

export function ApiDocsSection() {
  return (
    <div className="space-y-6">
      <div>
        <p className="text-sm text-muted-foreground">
          Connect your applications to {appConfig.name} using the REST API
        </p>
      </div>

      <Tabs defaultValue="auth" className="space-y-6">
        <TabsList className="bg-secondary border border-border p-1">
          <TabsTrigger value="auth" className="gap-1.5"><ShieldCheck className="w-3.5 h-3.5" /> Authentication</TabsTrigger>
          <TabsTrigger value="endpoints" className="gap-1.5"><Send className="w-3.5 h-3.5" /> Endpoints</TabsTrigger>
          <TabsTrigger value="examples" className="gap-1.5"><Zap className="w-3.5 h-3.5" /> Examples</TabsTrigger>
        </TabsList>

        {/* ---- Authentication Tab ---- */}
        <TabsContent value="auth" className="space-y-6 animate-in fade-in slide-in-from-bottom-2 duration-300">
          <Card className="border-border bg-card">
            <CardHeader>
              <CardTitle className="text-base font-medium flex items-center gap-2">
                <Key className="w-4 h-4 text-accent" />
                API Key Authentication
              </CardTitle>
            </CardHeader>
            <CardContent className="space-y-4">
              <p className="text-sm text-muted-foreground">
                All API requests require a valid API key passed via the <code className="text-xs bg-secondary px-1.5 py-0.5 rounded font-mono">Authorization</code> header.
                Create API keys in <span className="text-foreground font-medium">Settings &rarr; API Keys</span>.
              </p>
              <CopyBlock
                label="Header format"
                code="Authorization: Bearer pk_live_your_api_key_here"
              />
            </CardContent>
          </Card>

          <Card className="border-border bg-card">
            <CardHeader>
              <CardTitle className="text-base font-medium flex items-center gap-2">
                <Zap className="w-4 h-4 text-chart-3" />
                Rate Limiting
              </CardTitle>
            </CardHeader>
            <CardContent className="space-y-3">
              <p className="text-sm text-muted-foreground">
                API requests are rate-limited to <span className="text-foreground font-medium">60 requests per minute</span> per API key.
              </p>
              <p className="text-sm text-muted-foreground">
                When the limit is exceeded, the API returns <code className="text-xs bg-secondary px-1.5 py-0.5 rounded font-mono">429 Too Many Requests</code> with
                a <code className="text-xs bg-secondary px-1.5 py-0.5 rounded font-mono">Retry-After</code> header indicating seconds to wait.
              </p>
            </CardContent>
          </Card>

          <Card className="border-border bg-card">
            <CardHeader>
              <CardTitle className="text-base font-medium">Error Responses</CardTitle>
            </CardHeader>
            <CardContent>
              <div className="overflow-x-auto">
                <table className="w-full text-sm">
                  <thead>
                    <tr className="border-b border-border">
                      <th className="text-left py-2 pr-4 text-xs font-semibold text-muted-foreground uppercase">Status</th>
                      <th className="text-left py-2 pr-4 text-xs font-semibold text-muted-foreground uppercase">Error</th>
                      <th className="text-left py-2 text-xs font-semibold text-muted-foreground uppercase">Description</th>
                    </tr>
                  </thead>
                  <tbody>
                    {[
                      { status: "401", error: "missing_api_key", desc: "No Authorization header provided" },
                      { status: "401", error: "invalid_api_key", desc: "API key is invalid or has been revoked" },
                      { status: "429", error: "rate_limited", desc: "Too many requests, check Retry-After header" },
                      { status: "400", error: "validation error", desc: "Request body failed Zod schema validation" },
                      { status: "404", error: "Not found", desc: "Resource does not exist" },
                    ].map((row) => (
                      <tr key={row.error} className="border-b border-border last:border-0">
                        <td className="py-2 pr-4"><code className="text-xs bg-secondary px-1.5 py-0.5 rounded font-mono">{row.status}</code></td>
                        <td className="py-2 pr-4 font-mono text-xs text-foreground">{row.error}</td>
                        <td className="py-2 text-muted-foreground">{row.desc}</td>
                      </tr>
                    ))}
                  </tbody>
                </table>
              </div>
            </CardContent>
          </Card>
        </TabsContent>

        {/* ---- Endpoints Tab ---- */}
        <TabsContent value="endpoints" className="space-y-6 animate-in fade-in slide-in-from-bottom-2 duration-300">
          <p className="text-sm text-muted-foreground">
            Base URL: <code className="text-xs bg-secondary px-1.5 py-0.5 rounded font-mono text-foreground">/api/v1</code>
          </p>

          {/* Products */}
          <Card className="border-border bg-card">
            <CardHeader>
              <CardTitle className="text-base font-medium flex items-center gap-2">
                <Package className="w-4 h-4 text-accent" />
                Products
                <span className="text-[10px] font-medium text-muted-foreground bg-secondary px-1.5 py-0.5 rounded-full">read-only</span>
              </CardTitle>
            </CardHeader>
            <CardContent>
              <EndpointRow method="GET" path="/products" description="List all products" />
              <EndpointRow method="GET" path="/products/:id" description="Get a product by ID" />
            </CardContent>
          </Card>

          {/* Customers */}
          <Card className="border-border bg-card">
            <CardHeader>
              <CardTitle className="text-base font-medium flex items-center gap-2">
                <Building2 className="w-4 h-4 text-chart-1" />
                Customers
              </CardTitle>
            </CardHeader>
            <CardContent className="space-y-0">
              <EndpointRow method="GET" path="/customers" description="List customers with products" />
              <EndpointRow method="POST" path="/customers" description="Create a customer" />
              <EndpointRow method="GET" path="/customers/:id" description="Get a customer" />
              <EndpointRow method="PUT" path="/customers/:id" description="Update a customer" />
              <EndpointRow method="DELETE" path="/customers/:id" description="Delete a customer" />
              <div className="mt-3 pt-3 border-t border-border">
                <p className="text-xs text-muted-foreground mb-2">Required fields for POST:</p>
                <code className="text-xs font-mono text-muted-foreground">
                  name, industry, tier (Enterprise|Growth|Starter), location, contact, email, phone
                </code>
              </div>
            </CardContent>
          </Card>

          {/* Deals */}
          <Card className="border-border bg-card">
            <CardHeader>
              <CardTitle className="text-base font-medium flex items-center gap-2">
                <Handshake className="w-4 h-4 text-chart-3" />
                Deals
              </CardTitle>
            </CardHeader>
            <CardContent className="space-y-0">
              <EndpointRow method="GET" path="/deals" description="List deals" />
              <EndpointRow method="POST" path="/deals" description="Create a deal" />
              <EndpointRow method="GET" path="/deals/:id" description="Get a deal" />
              <EndpointRow method="PUT" path="/deals/:id" description="Update a deal" />
              <EndpointRow method="DELETE" path="/deals/:id" description="Delete a deal" />
              <div className="mt-3 pt-3 border-t border-border space-y-2">
                <p className="text-xs text-muted-foreground">
                  <span className="text-foreground font-medium">Filters:</span>{" "}
                  <code className="bg-secondary px-1 py-0.5 rounded font-mono">?type=licensed|saas|api</code>{" "}
                  <code className="bg-secondary px-1 py-0.5 rounded font-mono">?dealType=sale|trial|partner</code>
                </p>
                <p className="text-xs text-muted-foreground">
                  <span className="text-foreground font-medium">Auto-license:</span> When creating a deal with <code className="bg-secondary px-1 py-0.5 rounded font-mono">productType: &quot;licensed&quot;</code> and
                  no <code className="bg-secondary px-1 py-0.5 rounded font-mono">licenseKey</code>, a license is auto-generated (1-year expiry, or custom via <code className="bg-secondary px-1 py-0.5 rounded font-mono">licenseExpiresAt</code>).
                </p>
                <p className="text-xs text-muted-foreground">
                  <span className="text-foreground font-medium">Auto-populate:</span> If you provide <code className="bg-secondary px-1 py-0.5 rounded font-mono">customerId</code> or <code className="bg-secondary px-1 py-0.5 rounded font-mono">productId</code>,
                  the company/contact/email and productName/productType fields are auto-filled from the referenced records.
                </p>
              </div>
            </CardContent>
          </Card>

          {/* Licenses */}
          <Card className="border-border bg-card">
            <CardHeader>
              <CardTitle className="text-base font-medium flex items-center gap-2">
                <KeyRound className="w-4 h-4 text-chart-5" />
                Licenses
              </CardTitle>
            </CardHeader>
            <CardContent className="space-y-0">
              <EndpointRow method="GET" path="/licenses" description="List licenses" />
              <EndpointRow method="POST" path="/licenses" description="Create a license" />
              <EndpointRow method="PUT" path="/licenses/:key" description="Update license status/expiry" />
              <EndpointRow method="DELETE" path="/licenses/:key" description="Delete a license" />
              <EndpointRow method="POST" path="/licenses/verify" description="Verify license validity" />
              <div className="mt-3 pt-3 border-t border-border">
                <p className="text-xs text-muted-foreground">
                  <span className="text-foreground font-medium">Filter:</span>{" "}
                  <code className="bg-secondary px-1 py-0.5 rounded font-mono">?status=active|expired|revoked|suspended</code>
                </p>
              </div>
            </CardContent>
          </Card>
          {/* Billing: Subscriptions */}
          <Card className="border-border bg-card">
            <CardHeader>
              <CardTitle className="text-base font-medium flex items-center gap-2">
                <CreditCard className="w-4 h-4 text-chart-2" />
                Billing: Subscriptions
              </CardTitle>
            </CardHeader>
            <CardContent className="space-y-0">
              <EndpointRow method="GET" path="/billing/subscriptions" description="List subscriptions" />
              <EndpointRow method="POST" path="/billing/subscriptions" description="Create a subscription" />
              <EndpointRow method="GET" path="/billing/subscriptions/:id" description="Get a subscription" />
              <EndpointRow method="PUT" path="/billing/subscriptions/:id" description="Update subscription status" />
              <div className="mt-3 pt-3 border-t border-border">
                <p className="text-xs text-muted-foreground">
                  <span className="text-foreground font-medium">Filters:</span>{" "}
                  <code className="bg-secondary px-1 py-0.5 rounded font-mono">?status=active|paused|canceled|past_due|trialing</code>{" "}
                  <code className="bg-secondary px-1 py-0.5 rounded font-mono">?customerId=ID</code>
                </p>
              </div>
            </CardContent>
          </Card>

          {/* Billing: Invoices */}
          <Card className="border-border bg-card">
            <CardHeader>
              <CardTitle className="text-base font-medium flex items-center gap-2">
                <FileText className="w-4 h-4 text-chart-4" />
                Billing: Invoices
                <span className="text-[10px] font-medium text-muted-foreground bg-secondary px-1.5 py-0.5 rounded-full">read-only</span>
              </CardTitle>
            </CardHeader>
            <CardContent className="space-y-0">
              <EndpointRow method="GET" path="/billing/invoices" description="List invoices" />
              <EndpointRow method="GET" path="/billing/invoices/:id" description="Get invoice with items + payments" />
              <div className="mt-3 pt-3 border-t border-border">
                <p className="text-xs text-muted-foreground">
                  <span className="text-foreground font-medium">Filters:</span>{" "}
                  <code className="bg-secondary px-1 py-0.5 rounded font-mono">?status=draft|issued|paid|overdue|void</code>{" "}
                  <code className="bg-secondary px-1 py-0.5 rounded font-mono">?customerId=ID</code>
                </p>
              </div>
            </CardContent>
          </Card>

          {/* Billing: Usage */}
          <Card className="border-border bg-card">
            <CardHeader>
              <CardTitle className="text-base font-medium flex items-center gap-2">
                <Activity className="w-4 h-4 text-chart-3" />
                Billing: Usage Metering
              </CardTitle>
            </CardHeader>
            <CardContent className="space-y-0">
              <EndpointRow method="POST" path="/billing/usage" description="Send usage event(s)" />
              <EndpointRow method="GET" path="/billing/usage" description="Query aggregated usage" />
              <div className="mt-3 pt-3 border-t border-border space-y-2">
                <p className="text-xs text-muted-foreground">
                  <span className="text-foreground font-medium">Batch:</span> POST accepts a single event object or an array of events.
                </p>
                <p className="text-xs text-muted-foreground">
                  <span className="text-foreground font-medium">Query params:</span>{" "}
                  <code className="bg-secondary px-1 py-0.5 rounded font-mono">?subscriptionId=ID</code> (required){" "}
                  <code className="bg-secondary px-1 py-0.5 rounded font-mono">?metricName=api_calls</code>{" "}
                  <code className="bg-secondary px-1 py-0.5 rounded font-mono">?from=ISO&to=ISO</code>
                </p>
                <p className="text-xs text-muted-foreground">
                  <span className="text-foreground font-medium">Idempotency:</span> Use <code className="bg-secondary px-1 py-0.5 rounded font-mono">idempotencyKey</code> to prevent duplicate events.
                </p>
              </div>
            </CardContent>
          </Card>
        </TabsContent>

        {/* ---- Examples Tab ---- */}
        <TabsContent value="examples" className="space-y-6 animate-in fade-in slide-in-from-bottom-2 duration-300">
          <Card className="border-border bg-card">
            <CardHeader>
              <CardTitle className="text-base font-medium">1. Create a Customer and Record a Deal</CardTitle>
            </CardHeader>
            <CardContent className="space-y-4">
              <p className="text-sm text-muted-foreground">
                Create a customer, then record a licensed deal. The API auto-generates a license key.
              </p>
              <CopyBlock
                label="Create customer"
                code={`curl -X POST /api/v1/customers \\
  -H "Authorization: Bearer pk_live_YOUR_KEY" \\
  -H "Content-Type: application/json" \\
  -d '{
    "name": "Acme Corp",
    "industry": "Technology",
    "tier": "Enterprise",
    "location": "San Francisco, CA",
    "contact": "Jane Smith",
    "email": "jane@acme.com",
    "phone": "+1-555-0100"
  }'`}
              />
              <CopyBlock
                label="Create a deal (auto-generates license)"
                code={`curl -X POST /api/v1/deals \\
  -H "Authorization: Bearer pk_live_YOUR_KEY" \\
  -H "Content-Type: application/json" \\
  -d '{
    "customerId": "CUSTOMER_ID_FROM_ABOVE",
    "productId": "1",
    "value": 125000,
    "dealType": "sale",
    "date": "2026-02-15"
  }'

# Response includes auto-generated licenseKey`}
              />
            </CardContent>
          </Card>

          <Card className="border-border bg-card">
            <CardHeader>
              <CardTitle className="text-base font-medium">2. List and Verify a License</CardTitle>
            </CardHeader>
            <CardContent className="space-y-4">
              <CopyBlock
                label="List active licenses"
                code={`curl /api/v1/licenses?status=active \\
  -H "Authorization: Bearer pk_live_YOUR_KEY"`}
              />
              <CopyBlock
                label="Verify a license key"
                code={`curl -X POST /api/v1/licenses/verify \\
  -H "Authorization: Bearer pk_live_YOUR_KEY" \\
  -H "Content-Type: application/json" \\
  -d '{ "licenseKey": "K9RF-XHWN-3TBP-QM7J" }'

# Returns: { "valid": true, "license": { ... } }`}
              />
            </CardContent>
          </Card>

          <Card className="border-border bg-card">
            <CardHeader>
              <CardTitle className="text-base font-medium">3. Update a License Status</CardTitle>
            </CardHeader>
            <CardContent className="space-y-4">
              <p className="text-sm text-muted-foreground">
                Suspend, revoke, or update expiry on an existing license.
              </p>
              <CopyBlock
                label="Suspend a license"
                code={`curl -X PUT /api/v1/licenses/K9RF-XHWN-3TBP-QM7J \\
  -H "Authorization: Bearer pk_live_YOUR_KEY" \\
  -H "Content-Type: application/json" \\
  -d '{ "status": "suspended" }'`}
              />
              <CopyBlock
                label="Extend expiry"
                code={`curl -X PUT /api/v1/licenses/K9RF-XHWN-3TBP-QM7J \\
  -H "Authorization: Bearer pk_live_YOUR_KEY" \\
  -H "Content-Type: application/json" \\
  -d '{ "expiresAt": "2027-01-15" }'`}
              />
            </CardContent>
          </Card>

          <Card className="border-border bg-card">
            <CardHeader>
              <CardTitle className="text-base font-medium">4. Record a Trial</CardTitle>
            </CardHeader>
            <CardContent className="space-y-4">
              <p className="text-sm text-muted-foreground">
                Create a trial deal with a custom license expiry for evaluation access.
              </p>
              <CopyBlock
                code={`curl -X POST /api/v1/deals \\
  -H "Authorization: Bearer pk_live_YOUR_KEY" \\
  -H "Content-Type: application/json" \\
  -d '{
    "company": "Startup Inc",
    "contact": "Alex Chen",
    "email": "alex@startup.io",
    "value": 0,
    "productId": "3",
    "dealType": "trial",
    "date": "2026-02-15",
    "licenseExpiresAt": "2026-03-01",
    "notes": "14-day Pro Plan evaluation"
  }'`}
              />
            </CardContent>
          </Card>
          <Card className="border-border bg-card">
            <CardHeader>
              <CardTitle className="text-base font-medium">5. Create a Subscription</CardTitle>
            </CardHeader>
            <CardContent className="space-y-4">
              <p className="text-sm text-muted-foreground">
                Subscribe a customer to a pricing plan.
              </p>
              <CopyBlock
                code={`curl -X POST /api/v1/billing/subscriptions \\
  -H "Authorization: Bearer pk_live_YOUR_KEY" \\
  -H "Content-Type: application/json" \\
  -d '{
    "customerId": "CUSTOMER_ID",
    "planId": "PLAN_ID",
    "quantity": 5,
    "currentPeriodStart": "2026-02-01",
    "currentPeriodEnd": "2026-03-01"
  }'`}
              />
            </CardContent>
          </Card>

          <Card className="border-border bg-card">
            <CardHeader>
              <CardTitle className="text-base font-medium">6. Send Usage Events</CardTitle>
            </CardHeader>
            <CardContent className="space-y-4">
              <p className="text-sm text-muted-foreground">
                Report usage from your app for metered billing. Supports single or batch events.
              </p>
              <CopyBlock
                label="Single event"
                code={`curl -X POST /api/v1/billing/usage \\
  -H "Authorization: Bearer pk_live_YOUR_KEY" \\
  -H "Content-Type: application/json" \\
  -d '{
    "subscriptionId": "SUB_ID",
    "metricName": "api_calls",
    "value": 150,
    "idempotencyKey": "req-abc-123"
  }'`}
              />
              <CopyBlock
                label="Batch events"
                code={`curl -X POST /api/v1/billing/usage \\
  -H "Authorization: Bearer pk_live_YOUR_KEY" \\
  -H "Content-Type: application/json" \\
  -d '[
    { "subscriptionId": "SUB_ID", "metricName": "api_calls", "value": 100 },
    { "subscriptionId": "SUB_ID", "metricName": "api_calls", "value": 50 }
  ]'`}
              />
            </CardContent>
          </Card>

          <Card className="border-border bg-card">
            <CardHeader>
              <CardTitle className="text-base font-medium">7. Query Usage & List Invoices</CardTitle>
            </CardHeader>
            <CardContent className="space-y-4">
              <CopyBlock
                label="Get aggregated usage for a subscription"
                code={`curl "/api/v1/billing/usage?subscriptionId=SUB_ID&metricName=api_calls" \\
  -H "Authorization: Bearer pk_live_YOUR_KEY"

# Returns: [{ "metricName": "api_calls", "totalValue": 2400, "count": 48 }]`}
              />
              <CopyBlock
                label="List paid invoices for a customer"
                code={`curl "/api/v1/billing/invoices?customerId=CUST_ID&status=paid" \\
  -H "Authorization: Bearer pk_live_YOUR_KEY"`}
              />
            </CardContent>
          </Card>
        </TabsContent>
      </Tabs>
    </div>
  );
}

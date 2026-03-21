"use client";

import { useEffect, useMemo, useState } from "react";
import { Check, Copy, Key, PlayCircle, Send, ShieldCheck } from "lucide-react";

import { appConfig } from "@/lib/app-config";
import { cn } from "@/lib/utils";
import {
  API_ENDPOINTS,
  endpointGroups,
  endpointPathParams,
  type ApiAuth,
  type ApiEndpoint,
  type ApiScope,
} from "@/lib/api-docs/endpoints";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { Input } from "@/components/ui/input";
import { Tabs, TabsContent, TabsList, TabsTrigger } from "@/components/ui/tabs";
import { Textarea } from "@/components/ui/textarea";

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
        <pre className="p-4 text-xs font-mono text-foreground overflow-x-auto whitespace-pre-wrap break-words">{code}</pre>
        <button
          onClick={copy}
          className="absolute top-2 right-2 p-1.5 rounded-md bg-card border border-border text-muted-foreground hover:text-foreground opacity-0 group-hover:opacity-100 transition-all"
          aria-label="Copy"
        >
          {copied ? <Check className="w-3.5 h-3.5 text-accent" /> : <Copy className="w-3.5 h-3.5" />}
        </button>
      </div>
    </div>
  );
}

const methodColors: Record<string, string> = {
  GET: "bg-blue-500/10 text-blue-500 border-blue-500/20",
  POST: "bg-sky-500/10 text-sky-500 border-sky-500/20",
  PUT: "bg-amber-500/10 text-amber-500 border-amber-500/20",
  PATCH: "bg-orange-500/10 text-orange-500 border-orange-500/20",
  DELETE: "bg-red-500/10 text-red-500 border-red-500/20",
};

function MethodBadge({ method }: { method: string }) {
  return (
    <span className={cn("inline-flex items-center px-2 py-0.5 rounded text-[11px] font-semibold font-mono border", methodColors[method] ?? "")}>{method}</span>
  );
}

function authLabel(auth: ApiAuth): string {
  if (auth === "apiKey") return "API key";
  if (auth === "session") return "Session";
  return "Public";
}

function buildPath(pathTemplate: string, params: Record<string, string>): string {
  return pathTemplate.replace(/\{([a-zA-Z0-9_]+)\}/g, (_, key: string) => {
    const value = params[key]?.trim();
    return encodeURIComponent(value || `{${key}}`);
  });
}

function parseJsonExample(input?: string): unknown {
  if (!input?.trim()) return undefined;
  try {
    return JSON.parse(input);
  } catch {
    return input;
  }
}

function defaultBodyExample(endpoint: ApiEndpoint): unknown {
  const id = endpoint.id;

  if (endpoint.method === "GET" || endpoint.method === "DELETE") {
    return null;
  }

  if (id.includes("auth-login")) {
    return { email: "admin@rustbill.local", password: "admin123" };
  }
  if (id.includes("cron") || id.includes("lifecycle") || id.includes("backfill") || id.includes("sign") || id.includes("logout") || id.includes("webhooks-test")) {
    return {};
  }

  if (id.includes("products-create")) {
    return {
      name: "RantAI Enterprise",
      productType: "saas",
      target: "120000",
      revenue: "38000",
    };
  }
  if (id.includes("products-update")) {
    return { name: "RantAI Enterprise Plus", target: "150000" };
  }

  if (id.includes("customers-create")) {
    return {
      name: "Acme Corp",
      industry: "SaaS",
      tier: "Growth",
      location: "Jakarta",
      contact: "Ari Wijaya",
      email: "ari@acme.com",
      phone: "+62-21-555-0100",
    };
  }
  if (id.includes("customers-update")) {
    return { tier: "Enterprise", billingEmail: "billing@acme.com" };
  }

  if (id.includes("deals-create")) {
    return {
      customerId: "CUSTOMER_ID",
      productId: "PRODUCT_ID",
      value: 125000,
      dealType: "sale",
      date: "2026-03-20",
    };
  }
  if (id.includes("deals-update")) {
    return { notes: "Follow-up completed", value: 130000 };
  }

  if (id.includes("licenses-verify")) {
    return { key: "LIC-XXXX", deviceId: "device-1" };
  }
  if (id.includes("licenses-keypair-create")) {
    return {};
  }
  if (id.includes("licenses-create")) {
    return {
      customerId: "CUSTOMER_ID",
      productId: "PRODUCT_ID",
      customerName: "Acme Corp",
      productName: "RantAI Pro",
      maxActivations: 5,
    };
  }
  if (id.includes("licenses-update")) {
    return { status: "active", maxActivations: 10 };
  }

  if (id.includes("api-keys-create")) {
    return { name: "Server Integration", customerId: "CUSTOMER_ID" };
  }

  if (id.includes("settings-update")) {
    return {
      provider: "stripe",
      settings: {
        secretKey: "sk_live_xxx",
        webhookSecret: "whsec_xxx",
      },
    };
  }

  if (id.includes("plans-create")) {
    return {
      name: "Growth Monthly",
      billingPeriod: "monthly",
      basePrice: "99",
      currency: "USD",
    };
  }
  if (id.includes("plans-update")) {
    return { name: "Growth Monthly Plus", basePrice: "119" };
  }

  if (id.includes("subs-change-plan")) {
    return { planId: "NEW_PLAN_ID", idempotencyKey: "chg_001" };
  }
  if (id.includes("subs-create")) {
    return {
      customerId: "CUSTOMER_ID",
      planId: "PLAN_ID",
      quantity: 1,
      status: "active",
    };
  }
  if (id.includes("subs-update")) {
    return { cancelAtPeriodEnd: true };
  }

  if (id.includes("invoices-items-add")) {
    return { description: "Extra seat", quantity: 1, unitPrice: "19", currency: "USD" };
  }
  if (id.includes("invoices-create")) {
    return { customerId: "CUSTOMER_ID", currency: "USD" };
  }
  if (id.includes("invoices-update")) {
    return { status: "issued" };
  }

  if (id.includes("payments-create")) {
    return {
      invoiceId: "INVOICE_ID",
      amount: "100",
      currency: "USD",
      provider: "stripe",
      status: "succeeded",
    };
  }
  if (id.includes("payments-update")) {
    return { status: "succeeded", providerRef: "pi_123" };
  }

  if (id.includes("credit-notes-create")) {
    return { invoiceId: "INVOICE_ID", amount: "10", reason: "service_issue" };
  }
  if (id.includes("credit-notes-update")) {
    return { reason: "duplicate_charge", amount: "8" };
  }

  if (id.includes("coupons-create")) {
    return {
      code: "PROMO10",
      discountType: "percent",
      discountValue: "10",
      duration: "once",
    };
  }
  if (id.includes("coupons-update")) {
    return { active: false };
  }

  if (id.includes("refunds-create")) {
    return { paymentId: "PAYMENT_ID", amount: "5", reason: "requested_by_customer" };
  }
  if (id.includes("refunds-update")) {
    return { reason: "duplicate", status: "processed" };
  }

  if (id.includes("usage-create") || id.includes("usage-record")) {
    return {
      subscriptionId: "SUBSCRIPTION_ID",
      metricName: "api_calls",
      value: 120,
      idempotencyKey: "usage_evt_001",
    };
  }
  if (id.includes("usage-update")) {
    return { value: 140 };
  }

  if (id.includes("dunning-create")) {
    return { invoiceId: "INVOICE_ID", attemptCount: 1 };
  }

  if (id.includes("webhooks-create")) {
    return {
      provider: "stripe",
      url: "https://example.com/webhooks/billing",
      events: ["invoice.paid", "payment.failed"],
    };
  }
  if (id.includes("webhooks-update")) {
    return { active: true, events: ["invoice.paid"] };
  }

  if (id.includes("credits-adjust-update")) {
    return { amount: "15", description: "Correction" };
  }
  if (id.includes("credits-adjust")) {
    return {
      customerId: "CUSTOMER_ID",
      currency: "USD",
      amount: "20",
      description: "Manual goodwill credit",
    };
  }

  if (id.includes("tax-rules-create")) {
    return {
      countryCode: "US",
      regionCode: "CA",
      taxRate: "0.1025",
      taxName: "Sales Tax",
    };
  }
  if (id.includes("tax-rules-update")) {
    return { taxRate: "0.1050", active: true };
  }

  if (id.includes("pm-setup")) {
    return { customerId: "CUSTOMER_ID", provider: "stripe" };
  }
  if (id.includes("pm-default")) {
    return { customerId: "CUSTOMER_ID" };
  }
  if (id.includes("pm-create")) {
    return {
      customerId: "CUSTOMER_ID",
      provider: "stripe",
      providerToken: "pm_123",
      methodType: "card",
      label: "Visa ending 4242",
    };
  }

  return { idempotencyKey: "req_001" };
}

function endpointPayloadExample(endpoint: ApiEndpoint): string {
  const pathParamNames = endpointPathParams(endpoint.path);
  const pathParams = Object.fromEntries(pathParamNames.map((name) => [name, name.toUpperCase()]));
  const query = endpoint.queryHint?.replace(/^\?/, "") || null;
  const requestBody = parseJsonExample(endpoint.requestExample) ?? defaultBodyExample(endpoint);

  return JSON.stringify(
    {
      pathParams,
      query,
      body: requestBody,
    },
    null,
    2,
  );
}

type ApiTesterResult = {
  url: string;
  status: number;
  statusText: string;
  durationMs: number;
  headers: Record<string, string>;
  body: string;
};

function endpointCurl(endpoint: ApiEndpoint, url: string, apiKey: string, bodyInput: string): string {
  const parts = [`curl -X ${endpoint.method} "${url}"`];

  if (endpoint.auth === "apiKey") {
    parts.push(`  -H "Authorization: Bearer ${apiKey || "pk_live_your_api_key_here"}"`);
  }
  if (endpoint.auth === "session") {
    parts.push('  -b "session=your_session_cookie"');
  }

  const sendBody = endpoint.method !== "GET" && endpoint.method !== "DELETE" && bodyInput.trim().length > 0;
  if (sendBody) {
    parts.push('  -H "Content-Type: application/json"');
    parts.push(`  -d '${bodyInput}'`);
  }

  return parts.join(" \\\n+");
}

function EndpointRow({ endpoint }: { endpoint: ApiEndpoint }) {
  return (
    <div className="py-3 border-b border-border last:border-0 space-y-2">
      <div className="flex items-start gap-3">
        <MethodBadge method={endpoint.method} />
        <code className="text-sm font-mono text-foreground shrink-0">{endpoint.path}</code>
        <span className="text-xs text-muted-foreground border border-border rounded px-2 py-0.5">{authLabel(endpoint.auth)}</span>
        {endpoint.isSensitive && (
          <span className="text-xs text-amber-300 border border-amber-500/30 bg-amber-500/10 rounded px-2 py-0.5">sensitive</span>
        )}
        <span className="text-sm text-muted-foreground ml-auto text-right">{endpoint.description}</span>
      </div>

      <CopyBlock label="Example payload" code={endpointPayloadExample(endpoint)} />
    </div>
  );
}

export function ApiDocsSection() {
  const [scope, setScope] = useState<ApiScope>("public");
  const [selectedEndpointId, setSelectedEndpointId] = useState<string>("");
  const [apiKey, setApiKey] = useState("");
  const [pathParams, setPathParams] = useState<Record<string, string>>({});
  const [queryInput, setQueryInput] = useState("");
  const [bodyInput, setBodyInput] = useState("");
  const [result, setResult] = useState<ApiTesterResult | null>(null);
  const [requestError, setRequestError] = useState<string>("");
  const [isSending, setIsSending] = useState(false);

  const scopedEndpoints = useMemo(() => API_ENDPOINTS.filter((endpoint) => endpoint.scope === scope), [scope]);
  const selectedEndpoint = useMemo(() => scopedEndpoints.find((endpoint) => endpoint.id === selectedEndpointId) ?? scopedEndpoints[0], [selectedEndpointId, scopedEndpoints]);
  const groups = useMemo(() => endpointGroups(scope), [scope]);

  useEffect(() => {
    if (!selectedEndpoint) return;
    setSelectedEndpointId(selectedEndpoint.id);

    const nextParams: Record<string, string> = {};
    for (const key of endpointPathParams(selectedEndpoint.path)) {
      nextParams[key] = key.toUpperCase();
    }
    setPathParams(nextParams);

    setQueryInput(selectedEndpoint.queryHint?.replace(/^\?/, "") ?? "");
    const parsedBody = parseJsonExample(selectedEndpoint.requestExample) ?? defaultBodyExample(selectedEndpoint);
    if (selectedEndpoint.method === "GET" || selectedEndpoint.method === "DELETE" || parsedBody === null) {
      setBodyInput("");
    } else {
      setBodyInput(JSON.stringify(parsedBody, null, 2));
    }
    setRequestError("");
    setResult(null);
  }, [selectedEndpoint]);

  async function runRequest() {
    if (!selectedEndpoint) return;

    setRequestError("");
    setResult(null);

    const path = buildPath(selectedEndpoint.path, pathParams);
    const search = queryInput.trim() ? `?${queryInput.trim().replace(/^\?/, "")}` : "";
    const relativeUrl = `${path}${search}`;
    let requestBody: string | undefined;

    if (selectedEndpoint.method !== "GET" && selectedEndpoint.method !== "DELETE" && bodyInput.trim()) {
      try {
        requestBody = JSON.stringify(JSON.parse(bodyInput));
      } catch {
        setRequestError("Request body must be valid JSON.");
        return;
      }
    }

    if (selectedEndpoint.auth === "apiKey" && !apiKey.trim()) {
      setRequestError("This endpoint requires an API key.");
      return;
    }

    const headers = new Headers();
    if (selectedEndpoint.auth === "apiKey") {
      headers.set("Authorization", `Bearer ${apiKey.trim()}`);
    }
    if (requestBody) {
      headers.set("Content-Type", "application/json");
    }

    const start = performance.now();
    setIsSending(true);

    try {
      const response = await fetch(relativeUrl, {
        method: selectedEndpoint.method,
        headers,
        credentials: selectedEndpoint.auth === "session" ? "include" : "same-origin",
        body: requestBody,
      });

      const rawBody = await response.text();
      let prettyBody = rawBody;
      try {
        prettyBody = JSON.stringify(JSON.parse(rawBody), null, 2);
      } catch {
        // Keep plain text body for non-JSON responses.
      }

      const headerMap: Record<string, string> = {};
      for (const [key, value] of response.headers.entries()) {
        headerMap[key] = value;
      }

      setResult({
        url: relativeUrl,
        status: response.status,
        statusText: response.statusText,
        durationMs: Math.round(performance.now() - start),
        headers: headerMap,
        body: prettyBody || "(empty response)",
      });
    } catch {
      setRequestError("Request failed before receiving a response.");
    } finally {
      setIsSending(false);
    }
  }

  return (
    <div className="space-y-6">
      <div>
        <p className="text-sm text-muted-foreground">Explore and test {appConfig.name} APIs from the dashboard.</p>
      </div>

      <Tabs defaultValue="auth" className="space-y-6">
        <TabsList className="bg-secondary border border-border p-1">
          <TabsTrigger value="auth" className="gap-1.5"><ShieldCheck className="w-3.5 h-3.5" /> Authentication</TabsTrigger>
          <TabsTrigger value="endpoints" className="gap-1.5"><Send className="w-3.5 h-3.5" /> Endpoints</TabsTrigger>
          <TabsTrigger value="examples" className="gap-1.5"><Key className="w-3.5 h-3.5" /> Examples</TabsTrigger>
          <TabsTrigger value="playground" className="gap-1.5"><PlayCircle className="w-3.5 h-3.5" /> Playground</TabsTrigger>
        </TabsList>

        <TabsContent value="auth" className="space-y-6 animate-in fade-in slide-in-from-bottom-2 duration-300">
          <Card className="border-border bg-card">
            <CardHeader>
              <CardTitle className="text-base font-medium">Public API auth (/api/v1/*)</CardTitle>
            </CardHeader>
            <CardContent className="space-y-4">
              <p className="text-sm text-muted-foreground">Use an API key with Bearer auth. Generate keys in <span className="text-foreground font-medium">Settings → API Keys</span>.</p>
              <CopyBlock code="Authorization: Bearer pk_live_your_api_key_here" label="Header format" />
            </CardContent>
          </Card>

          <Card className="border-border bg-card">
            <CardHeader>
              <CardTitle className="text-base font-medium">Admin API auth (/api/*)</CardTitle>
            </CardHeader>
            <CardContent className="space-y-3">
              <p className="text-sm text-muted-foreground">Admin endpoints require an authenticated session cookie.</p>
              <CopyBlock code={`POST /api/auth/login\n{\n  "email": "admin@rustbill.local",\n  "password": "admin123"\n}\n\n# Then call /api/* with the returned session cookie.`} />
            </CardContent>
          </Card>
        </TabsContent>

        <TabsContent value="endpoints" className="space-y-6 animate-in fade-in slide-in-from-bottom-2 duration-300">
          <div className="flex items-center gap-2">
            <Button variant={scope === "public" ? "default" : "outline"} size="sm" onClick={() => setScope("public")}>Public API</Button>
            <Button variant={scope === "admin" ? "default" : "outline"} size="sm" onClick={() => setScope("admin")}>Admin API</Button>
          </div>

          {groups.map((group) => (
            <Card className="border-border bg-card" key={`${scope}-${group}`}>
              <CardHeader>
                <CardTitle className="text-base font-medium">{group}</CardTitle>
              </CardHeader>
              <CardContent>
                {scopedEndpoints
                  .filter((endpoint) => endpoint.group === group)
                  .map((endpoint) => (
                    <EndpointRow endpoint={endpoint} key={endpoint.id} />
                  ))}
              </CardContent>
            </Card>
          ))}
        </TabsContent>

        <TabsContent value="examples" className="space-y-6 animate-in fade-in slide-in-from-bottom-2 duration-300">
          <Card className="border-border bg-card">
            <CardHeader>
              <CardTitle className="text-base font-medium">Public v1 example</CardTitle>
            </CardHeader>
            <CardContent className="space-y-4">
              <CopyBlock
                label="Verify a license"
                code={`curl -X POST /api/v1/licenses/verify \\
  -H "Authorization: Bearer pk_live_YOUR_KEY" \\
  -H "Content-Type: application/json" \\
  -d '{\n+    "key": "LIC-XXXX",\n+    "deviceId": "device-1"\n+  }'`}
              />
            </CardContent>
          </Card>

          <Card className="border-border bg-card">
            <CardHeader>
              <CardTitle className="text-base font-medium">Admin API example</CardTitle>
            </CardHeader>
            <CardContent className="space-y-4">
              <CopyBlock
                label="Login and call admin endpoint"
                code={`curl -X POST /api/auth/login \\
  -H "Content-Type: application/json" \\
  -d '{\n+    "email": "admin@rustbill.local",\n+    "password": "admin123"\n+  }'\n\n# Reuse returned session cookie\ncurl /api/products -b "session=YOUR_SESSION_COOKIE"`}
              />
            </CardContent>
          </Card>
        </TabsContent>

        <TabsContent value="playground" className="space-y-6 animate-in fade-in slide-in-from-bottom-2 duration-300">
          <Card className="border-border bg-card">
            <CardHeader>
              <CardTitle className="text-base font-medium">API Playground</CardTitle>
            </CardHeader>
            <CardContent className="space-y-4">
              <div className="grid gap-3 md:grid-cols-2">
                <div className="space-y-1.5">
                  <label htmlFor="api-scope" className="text-xs text-muted-foreground">Scope</label>
                  <select
                    id="api-scope"
                    className="border-input bg-transparent h-9 w-full rounded-md border px-3 text-sm"
                    value={scope}
                    onChange={(event) => setScope(event.target.value as ApiScope)}
                  >
                    <option value="public">Public API (/api/v1)</option>
                    <option value="admin">Admin API (/api)</option>
                  </select>
                </div>

                <div className="space-y-1.5">
                  <label htmlFor="api-endpoint" className="text-xs text-muted-foreground">Endpoint</label>
                  <select
                    id="api-endpoint"
                    aria-label="Endpoint"
                    className="border-input bg-transparent h-9 w-full rounded-md border px-3 text-sm"
                    value={selectedEndpoint?.id ?? ""}
                    onChange={(event) => setSelectedEndpointId(event.target.value)}
                  >
                    {scopedEndpoints.map((endpoint) => (
                      <option value={endpoint.id} key={endpoint.id}>
                        [{endpoint.method}] {endpoint.path}
                      </option>
                    ))}
                  </select>
                </div>
              </div>

              {selectedEndpoint?.auth === "apiKey" && (
                <div className="space-y-1.5">
                  <label htmlFor="api-key" className="text-xs text-muted-foreground">API key (Bearer)</label>
                  <Input id="api-key" aria-label="API key" value={apiKey} onChange={(event) => setApiKey(event.target.value)} placeholder="pk_live_..." />
                </div>
              )}

              <div className="grid gap-3 md:grid-cols-2">
                {selectedEndpoint && endpointPathParams(selectedEndpoint.path).map((key) => (
                  <div key={key} className="space-y-1.5">
                    <label htmlFor={`path-param-${key}`} className="text-xs text-muted-foreground">Path param: {key}</label>
                    <Input
                      id={`path-param-${key}`}
                      value={pathParams[key] ?? ""}
                      onChange={(event) => setPathParams((current) => ({ ...current, [key]: event.target.value }))}
                    />
                  </div>
                ))}
                <div className="space-y-1.5 md:col-span-2">
                  <label htmlFor="query-string" className="text-xs text-muted-foreground">Query string (without ?)</label>
                  <Input id="query-string" aria-label="Query string" value={queryInput} onChange={(event) => setQueryInput(event.target.value)} placeholder="status=active&limit=20" />
                </div>
              </div>

              <div className="space-y-1.5">
                <label htmlFor="request-body" className="text-xs text-muted-foreground">JSON body</label>
                <Textarea id="request-body" aria-label="JSON body" value={bodyInput} onChange={(event) => setBodyInput(event.target.value)} className="min-h-44 font-mono text-xs" />
              </div>

              <div className="flex items-center gap-2">
                <Button onClick={runRequest} disabled={isSending || !selectedEndpoint}>
                  {isSending ? "Sending..." : "Send request"}
                </Button>
                {selectedEndpoint && (
                  <span className="text-xs text-muted-foreground">Auth: {authLabel(selectedEndpoint.auth)}</span>
                )}
              </div>

              {selectedEndpoint?.isSensitive && (
                <p className="text-xs text-amber-300/90 border border-amber-500/20 bg-amber-500/10 rounded-md px-3 py-2">
                  Sensitive endpoint: this can trigger billing jobs, backfills, or system-wide state changes.
                </p>
              )}

              {requestError && <p className="text-sm text-red-400">{requestError}</p>}
            </CardContent>
          </Card>

          {result && selectedEndpoint && (
            <Card className="border-border bg-card">
              <CardHeader>
                <CardTitle className="text-base font-medium">Response</CardTitle>
              </CardHeader>
              <CardContent className="space-y-4">
                <p className="text-sm text-muted-foreground">
                  <span className="text-foreground font-medium">{result.status}</span> {result.statusText} · {result.durationMs} ms
                </p>
                <CopyBlock label="Request URL" code={result.url} />
                <CopyBlock label="Response headers" code={JSON.stringify(result.headers, null, 2)} />
                <CopyBlock label="Response body" code={result.body} />
                <CopyBlock
                  label="cURL"
                  code={endpointCurl(
                    selectedEndpoint,
                    typeof window !== "undefined" ? `${window.location.origin}${result.url}` : result.url,
                    apiKey,
                    bodyInput,
                  )}
                />
              </CardContent>
            </Card>
          )}
        </TabsContent>
      </Tabs>
    </div>
  );
}

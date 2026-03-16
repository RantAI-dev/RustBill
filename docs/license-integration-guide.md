# License Integration Guide

This guide covers how to integrate license verification into your application installer or runtime. Two models are supported — choose based on your deployment environment.

| Model | Requires Internet | Revocation | Best For |
|-------|-------------------|------------|----------|
| **Online** | Yes | Instant | SaaS apps, desktop apps with connectivity |
| **Offline** | No | Not real-time | Air-gapped environments, embedded systems, on-premise |

---

## Key Concepts

There are two different "keys" in this system — don't confuse them:

| | API Key | License Key |
|---|---|---|
| **Looks like** | `pk_live_a1b2c3d4e5f6...` | `ABCD-EFGH-IJKL-MNOP` |
| **Who sees it** | Only your app (hidden from user) | The customer |
| **Purpose** | Authenticates your app to the server | Identifies the customer's license |
| **Where it lives** | Embedded & encrypted in your app binary | Customer enters it manually |
| **Created in** | Settings > API Keys | Auto-generated when a deal is won |

The customer **never** enters or sees the API key. Your app uses it internally to talk to the server.

---

## Prerequisites

### For Online Licensing
1. Create an API key in **Settings > API Keys** in the dashboard
2. Embed the API key in your app binary (encrypted/obfuscated — see Security Notes below)
3. Note your server URL (e.g. `https://your-dashboard.com`)

### For Offline Licensing
1. Generate a signing keypair in **Settings > License Signing**
2. Download the public key `.pem` file and bundle it with your application
3. Sign licenses from **License Management > Generate Certificate** or they auto-sign when a deal is created

---

## 1. Online Licensing

The customer enters their license key. Your app uses its embedded API key to verify it against the server.

### Flow

```
Customer enters license key (XXXX-XXXX-XXXX-XXXX)
        │
        ▼
App sends to server using embedded API key (hidden from user):
  POST /api/v1/licenses/verify
  Authorization: Bearer pk_live_...     ← embedded in your app, encrypted
  { "licenseKey": "XXXX-XXXX-..." }    ← from customer input
        │
        ▼
Server validates key in DB
        │
  ┌─────┴─────┐
valid       invalid
  │             │
Activate    Show error
```

### API Endpoint

```
POST /api/v1/licenses/verify
Authorization: Bearer <your-api-key>
Content-Type: application/json

{ "licenseKey": "ABCD-EFGH-IJKL-MNOP" }
```

### Response — Valid License

```json
{
  "valid": true,
  "license": {
    "key": "ABCD-EFGH-IJKL-MNOP",
    "status": "active",
    "product": "Desktop Pro",
    "productId": "prod-uuid",
    "customer": "Acme Corp",
    "customerId": "cust-uuid",
    "createdAt": "2026-01-15",
    "expiresAt": "2027-01-15"
  }
}
```

### Response — Invalid License

```json
{ "valid": false, "error": "not_found", "message": "License key not found" }
```

```json
{ "valid": false, "error": "license_inactive", "status": "revoked", "message": "License is revoked" }
```

```json
{ "valid": false, "error": "license_expired", "status": "expired", "expiresAt": "2025-06-01", "message": "License has expired" }
```

### Response — Auth Errors

```json
{ "error": "missing_api_key", "message": "Authorization header with Bearer token required" }
```

```json
{ "error": "invalid_api_key", "message": "API key is invalid or revoked" }
```

```json
{ "error": "rate_limited", "message": "Too many requests", "retryAfter": 60 }
```

### Example — Node.js / TypeScript

```typescript
const API_URL = "https://your-dashboard.com";
const API_KEY = "pk_live_..."; // from Settings > API Keys

async function verifyLicense(licenseKey: string): Promise<{
  valid: boolean;
  license?: { product: string; expiresAt: string };
  error?: string;
}> {
  const res = await fetch(`${API_URL}/api/v1/licenses/verify`, {
    method: "POST",
    headers: {
      "Authorization": `Bearer ${API_KEY}`,
      "Content-Type": "application/json",
    },
    body: JSON.stringify({ licenseKey }),
  });

  return res.json();
}

// Usage in your installer/app
const result = await verifyLicense("ABCD-EFGH-IJKL-MNOP");
if (result.valid) {
  console.log(`Licensed for: ${result.license.product}`);
  console.log(`Valid until: ${result.license.expiresAt}`);
  // Proceed with activation
} else {
  console.error(`License invalid: ${result.error}`);
  // Show error to user
}
```

### Example — Python

```python
import requests

API_URL = "https://your-dashboard.com"
API_KEY = "pk_live_..."

def verify_license(license_key: str) -> dict:
    res = requests.post(
        f"{API_URL}/api/v1/licenses/verify",
        headers={
            "Authorization": f"Bearer {API_KEY}",
            "Content-Type": "application/json",
        },
        json={"licenseKey": license_key},
    )
    return res.json()

# Usage
result = verify_license("ABCD-EFGH-IJKL-MNOP")
if result["valid"]:
    print(f"Licensed for: {result['license']['product']}")
    print(f"Valid until: {result['license']['expiresAt']}")
else:
    print(f"License invalid: {result['error']}")
```

### Example — Go

```go
package main

import (
    "bytes"
    "encoding/json"
    "fmt"
    "net/http"
)

const apiURL = "https://your-dashboard.com"
const apiKey = "pk_live_..."

type VerifyRequest struct {
    LicenseKey string `json:"licenseKey"`
}

type License struct {
    Key       string `json:"key"`
    Status    string `json:"status"`
    Product   string `json:"product"`
    ExpiresAt string `json:"expiresAt"`
}

type VerifyResponse struct {
    Valid   bool    `json:"valid"`
    License License `json:"license"`
    Error   string  `json:"error"`
    Message string  `json:"message"`
}

func verifyLicense(licenseKey string) (*VerifyResponse, error) {
    body, _ := json.Marshal(VerifyRequest{LicenseKey: licenseKey})
    req, _ := http.NewRequest("POST", apiURL+"/api/v1/licenses/verify", bytes.NewReader(body))
    req.Header.Set("Authorization", "Bearer "+apiKey)
    req.Header.Set("Content-Type", "application/json")

    resp, err := http.DefaultClient.Do(req)
    if err != nil {
        return nil, err
    }
    defer resp.Body.Close()

    var result VerifyResponse
    json.NewDecoder(resp.Body).Decode(&result)
    return &result, nil
}

func main() {
    result, err := verifyLicense("ABCD-EFGH-IJKL-MNOP")
    if err != nil {
        fmt.Println("Network error:", err)
        return
    }
    if result.Valid {
        fmt.Printf("Licensed for: %s (until %s)\n", result.License.Product, result.License.ExpiresAt)
    } else {
        fmt.Printf("Invalid: %s\n", result.Message)
    }
}
```

### Example — C# / .NET

```csharp
using System.Net.Http;
using System.Net.Http.Headers;
using System.Text;
using System.Text.Json;

const string ApiUrl = "https://your-dashboard.com";
const string ApiKey = "pk_live_...";

var client = new HttpClient();
client.DefaultRequestHeaders.Authorization = new AuthenticationHeaderValue("Bearer", ApiKey);

var content = new StringContent(
    JsonSerializer.Serialize(new { licenseKey = "ABCD-EFGH-IJKL-MNOP" }),
    Encoding.UTF8, "application/json");

var response = await client.PostAsync($"{ApiUrl}/api/v1/licenses/verify", content);
var json = await response.Content.ReadAsStringAsync();
var result = JsonSerializer.Deserialize<JsonElement>(json);

if (result.GetProperty("valid").GetBoolean())
{
    var license = result.GetProperty("license");
    Console.WriteLine($"Licensed for: {license.GetProperty("product")}");
    Console.WriteLine($"Valid until: {license.GetProperty("expiresAt")}");
}
else
{
    Console.WriteLine($"Invalid: {result.GetProperty("message")}");
}
```

### Activation Tracking (Device Limits)

To prevent a single license key from being used on unlimited devices, include a `deviceId` in your verify request. The server tracks each unique device and enforces the `maxActivations` limit set on the license.

#### Request with Device Tracking

```
POST /api/v1/licenses/verify
Authorization: Bearer <your-api-key>
Content-Type: application/json

{
  "licenseKey": "ABCD-EFGH-IJKL-MNOP",
  "deviceId": "a1b2c3d4-hardware-fingerprint",
  "deviceName": "John's MacBook Pro"
}
```

- **`deviceId`** (optional) — a unique, stable identifier for the device. Use a hardware fingerprint, MAC address hash, or UUID persisted to disk on first run.
- **`deviceName`** (optional) — a human-readable label shown in the admin dashboard.

#### Response with Activation Info

```json
{
  "valid": true,
  "license": { "key": "...", "status": "active", "product": "...", "features": ["pro"] },
  "activations": 2,
  "maxActivations": 5
}
```

#### Activation Limit Reached

```json
{
  "valid": false,
  "error": "activation_limit_reached",
  "message": "Maximum 5 activations reached",
  "activations": 5,
  "maxActivations": 5
}
```

When a customer hits the limit, they must contact the admin. The admin can deactivate old devices from **License Management > License Details > Activations**.

#### Generating a Device ID

```typescript
import crypto from "node:crypto";
import os from "node:os";
import fs from "node:fs";

function getDeviceId(): string {
  const idFile = path.join(os.homedir(), ".yourapp", "device-id");

  // Try to read existing device ID
  try {
    return fs.readFileSync(idFile, "utf-8").trim();
  } catch {
    // Generate a new one from hardware info
    const info = [os.hostname(), os.platform(), os.arch(), os.cpus()[0]?.model].join("|");
    const id = crypto.createHash("sha256").update(info).digest("hex").slice(0, 32);
    fs.mkdirSync(path.dirname(idFile), { recursive: true });
    fs.writeFileSync(idFile, id);
    return id;
  }
}
```

#### Listing Activations (Public API)

Apps can show users how many devices are activated:

```
GET /api/v1/licenses/{key}/activations
Authorization: Bearer <your-api-key>
```

```json
{
  "activations": [
    { "deviceId": "a1b2c3...", "deviceName": "John's MacBook Pro", "activatedAt": "...", "lastSeenAt": "..." },
    { "deviceId": "d4e5f6...", "deviceName": "Office Desktop", "activatedAt": "...", "lastSeenAt": "..." }
  ],
  "count": 2,
  "maxActivations": 5
}
```

> **Backward compatible**: If you omit `deviceId`, the endpoint works exactly as before — simple validation with no activation tracking.

### Best Practices — Online

- **Encrypt the embedded API key** — never store it as plaintext in your binary. Use compile-time encryption with a runtime decryption routine, or load it from an encrypted config file. This prevents trivial extraction via `strings` or a hex editor.
- **Consider a backend proxy** — for maximum security, route verification through your own backend instead of calling the dashboard API directly from the client app. This way the API key never ships in the binary at all.
- **Cache the result locally** after a successful verification (e.g. 24h) so the app doesn't block on every startup if the server is temporarily unreachable.
- **Handle network errors gracefully** — if the server is unreachable, fall back to a cached verification or show "unable to verify, running in grace period".
- **Re-verify periodically** — check the license on app startup and optionally on a timer (e.g. every 24h) to catch revocations and expirations.
- **Always send `deviceId`** — even if `maxActivations` isn't set, sending a device ID lets you track where licenses are being used and enables per-device management from the dashboard.

---

## 2. Offline Licensing (Air-Gapped)

The customer receives a `.lic` file (cryptographically signed certificate). Your app verifies it locally using the embedded public key — no internet required.

### Flow

```
Dashboard signs license ──> Customer gets .lic file (USB/email)
                                      │
                                      ▼
                            App reads .lic file
                                      │
                                      ▼
                      Verify Ed25519 signature with public key
                                      │
                            ┌─────────┴─────────┐
                        Signature valid      Signature invalid
                            │                      │
                      Check expiry date      Reject / show error
                            │
                      Activate app
```

### .lic File Format

```
-----BEGIN LICENSE-----
eyJsaWNlbnNlSWQiOiJBQkNELUVGR0gtSUpLTC1NTk9QIiwiY3Vz...  (base64-encoded JSON payload)
-----END LICENSE-----
-----BEGIN SIGNATURE-----
m3Ks8jL2p...  (base64-encoded Ed25519 signature)
-----END SIGNATURE-----
```

### License Payload (decoded from base64)

```json
{
  "licenseId": "ABCD-EFGH-IJKL-MNOP",
  "customerId": "cust-uuid",
  "customerName": "Acme Corp",
  "productId": "prod-uuid",
  "productName": "Desktop Pro",
  "features": ["pro", "enterprise", "custom-reports"],
  "maxActivations": 5,
  "issuedAt": "2026-01-15",
  "expiresAt": "2027-01-15"
}
```

### Verification Steps

Your app must:

1. **Parse** the `.lic` file to extract the base64 payload and signature
2. **Decode** the payload from base64 to get the JSON string
3. **Verify** the Ed25519 signature against the JSON string using the public key
4. **Check expiration** — compare `expiresAt` against the current date
5. **Check features** — optionally gate functionality based on the `features` array

> The public key is safe to embed in your application binary. It can only verify signatures, not create them.

### Example — Node.js / TypeScript

```typescript
import crypto from "node:crypto";
import fs from "node:fs";

// Bundle this with your app — downloaded from Settings > License Signing
const PUBLIC_KEY = `-----BEGIN PUBLIC KEY-----
MCowBQYDK2VwAyEA...
-----END PUBLIC KEY-----`;

interface LicensePayload {
  licenseId: string;
  customerId: string;
  customerName: string;
  productId: string;
  productName: string;
  features: string[];
  maxActivations?: number;
  issuedAt: string;
  expiresAt: string;
}

interface VerifyResult {
  valid: boolean;
  expired: boolean;
  payload: LicensePayload | null;
  error?: string;
}

function verifyLicenseFile(filePath: string): VerifyResult {
  try {
    const content = fs.readFileSync(filePath, "utf-8");

    // 1. Parse the .lic file
    const payloadMatch = content.match(
      /-----BEGIN LICENSE-----\n([\s\S]+?)\n-----END LICENSE-----/
    );
    const sigMatch = content.match(
      /-----BEGIN SIGNATURE-----\n([\s\S]+?)\n-----END SIGNATURE-----/
    );

    if (!payloadMatch || !sigMatch) {
      return { valid: false, expired: false, payload: null, error: "Invalid license file format" };
    }

    // 2. Decode the payload
    const payloadJson = Buffer.from(payloadMatch[1].trim(), "base64").toString("utf-8");
    const payload: LicensePayload = JSON.parse(payloadJson);

    // 3. Verify Ed25519 signature
    const signatureValid = crypto.verify(
      null, // Ed25519 uses intrinsic hash
      Buffer.from(payloadJson),
      PUBLIC_KEY,
      Buffer.from(sigMatch[1].trim(), "base64")
    );

    if (!signatureValid) {
      return { valid: false, expired: false, payload: null, error: "Invalid signature — license may be tampered" };
    }

    // 4. Check expiration
    const now = new Date().toISOString().split("T")[0];
    const expired = payload.expiresAt < now;

    return { valid: true, expired, payload };
  } catch {
    return { valid: false, expired: false, payload: null, error: "Failed to read or parse license file" };
  }
}

// Usage in your installer/app
const result = verifyLicenseFile("/path/to/license.lic");

if (!result.valid) {
  console.error(`License rejected: ${result.error}`);
  process.exit(1);
}

if (result.expired) {
  console.error(`License expired on ${result.payload!.expiresAt}`);
  process.exit(1);
}

console.log(`Licensed to: ${result.payload!.customerName}`);
console.log(`Product: ${result.payload!.productName}`);
console.log(`Features: ${result.payload!.features.join(", ")}`);
console.log(`Valid until: ${result.payload!.expiresAt}`);

// Feature gating
if (result.payload!.features.includes("enterprise")) {
  // Enable enterprise features
}
```

### Example — Python

```python
import json
import base64
from datetime import date
from cryptography.hazmat.primitives.asymmetric.ed25519 import Ed25519PublicKey
from cryptography.hazmat.primitives.serialization import load_pem_public_key

# pip install cryptography

# Bundle this with your app
PUBLIC_KEY_PEM = b"""-----BEGIN PUBLIC KEY-----
MCowBQYDK2VwAyEA...
-----END PUBLIC KEY-----"""


def verify_license_file(file_path: str) -> dict:
    try:
        with open(file_path, "r") as f:
            content = f.read()

        # 1. Parse
        import re
        payload_match = re.search(
            r"-----BEGIN LICENSE-----\n([\s\S]+?)\n-----END LICENSE-----", content
        )
        sig_match = re.search(
            r"-----BEGIN SIGNATURE-----\n([\s\S]+?)\n-----END SIGNATURE-----", content
        )

        if not payload_match or not sig_match:
            return {"valid": False, "error": "Invalid license file format"}

        # 2. Decode
        payload_json = base64.b64decode(payload_match.group(1).strip())
        payload = json.loads(payload_json)
        signature = base64.b64decode(sig_match.group(1).strip())

        # 3. Verify Ed25519 signature
        public_key = load_pem_public_key(PUBLIC_KEY_PEM)
        public_key.verify(signature, payload_json)  # raises on failure

        # 4. Check expiration
        expired = payload["expiresAt"] < date.today().isoformat()

        return {"valid": True, "expired": expired, "payload": payload}

    except Exception as e:
        return {"valid": False, "error": str(e)}


# Usage
result = verify_license_file("/path/to/license.lic")

if not result["valid"]:
    print(f"License rejected: {result['error']}")
    exit(1)

if result["expired"]:
    print(f"License expired on {result['payload']['expiresAt']}")
    exit(1)

payload = result["payload"]
print(f"Licensed to: {payload['customerName']}")
print(f"Features: {', '.join(payload['features'])}")
```

### Example — Go

```go
package main

import (
    "crypto/ed25519"
    "crypto/x509"
    "encoding/base64"
    "encoding/json"
    "encoding/pem"
    "fmt"
    "os"
    "regexp"
    "time"
)

// Bundle this with your app
const publicKeyPEM = `-----BEGIN PUBLIC KEY-----
MCowBQYDK2VwAyEA...
-----END PUBLIC KEY-----`

type LicensePayload struct {
    LicenseID      string   `json:"licenseId"`
    CustomerName   string   `json:"customerName"`
    ProductName    string   `json:"productName"`
    Features       []string `json:"features"`
    MaxActivations int      `json:"maxActivations,omitempty"`
    ExpiresAt      string   `json:"expiresAt"`
}

func verifyLicenseFile(filePath string) (*LicensePayload, error) {
    content, err := os.ReadFile(filePath)
    if err != nil {
        return nil, fmt.Errorf("cannot read file: %w", err)
    }

    // 1. Parse
    payloadRe := regexp.MustCompile(`(?s)-----BEGIN LICENSE-----\n(.+?)\n-----END LICENSE-----`)
    sigRe := regexp.MustCompile(`(?s)-----BEGIN SIGNATURE-----\n(.+?)\n-----END SIGNATURE-----`)

    payloadMatch := payloadRe.FindSubmatch(content)
    sigMatch := sigRe.FindSubmatch(content)
    if payloadMatch == nil || sigMatch == nil {
        return nil, fmt.Errorf("invalid license file format")
    }

    // 2. Decode
    payloadJSON, err := base64.StdEncoding.DecodeString(string(payloadMatch[1]))
    if err != nil {
        return nil, fmt.Errorf("invalid payload encoding: %w", err)
    }

    signature, err := base64.StdEncoding.DecodeString(string(sigMatch[1]))
    if err != nil {
        return nil, fmt.Errorf("invalid signature encoding: %w", err)
    }

    // 3. Verify Ed25519 signature
    block, _ := pem.Decode([]byte(publicKeyPEM))
    pub, err := x509.ParsePKIXPublicKey(block.Bytes)
    if err != nil {
        return nil, fmt.Errorf("invalid public key: %w", err)
    }
    edPub := pub.(ed25519.PublicKey)

    if !ed25519.Verify(edPub, payloadJSON, signature) {
        return nil, fmt.Errorf("invalid signature — license may be tampered")
    }

    // 4. Parse payload
    var payload LicensePayload
    if err := json.Unmarshal(payloadJSON, &payload); err != nil {
        return nil, fmt.Errorf("invalid payload: %w", err)
    }

    // 5. Check expiration
    expires, _ := time.Parse("2006-01-02", payload.ExpiresAt)
    if time.Now().After(expires) {
        return nil, fmt.Errorf("license expired on %s", payload.ExpiresAt)
    }

    return &payload, nil
}

func main() {
    payload, err := verifyLicenseFile("/path/to/license.lic")
    if err != nil {
        fmt.Println("License rejected:", err)
        os.Exit(1)
    }

    fmt.Printf("Licensed to: %s\n", payload.CustomerName)
    fmt.Printf("Product: %s\n", payload.ProductName)
    fmt.Printf("Features: %v\n", payload.Features)
}
```

### Example — C# / .NET

```csharp
using System.Security.Cryptography;
using System.Text;
using System.Text.Json;
using System.Text.RegularExpressions;

// Bundle this with your app
const string PublicKeyPem = @"-----BEGIN PUBLIC KEY-----
MCowBQYDK2VwAyEA...
-----END PUBLIC KEY-----";

static (bool valid, JsonElement? payload, string? error) VerifyLicenseFile(string filePath)
{
    try
    {
        var content = File.ReadAllText(filePath);

        // 1. Parse
        var payloadMatch = Regex.Match(content, @"-----BEGIN LICENSE-----\n([\s\S]+?)\n-----END LICENSE-----");
        var sigMatch = Regex.Match(content, @"-----BEGIN SIGNATURE-----\n([\s\S]+?)\n-----END SIGNATURE-----");

        if (!payloadMatch.Success || !sigMatch.Success)
            return (false, null, "Invalid license file format");

        // 2. Decode
        var payloadJson = Encoding.UTF8.GetString(Convert.FromBase64String(payloadMatch.Groups[1].Value.Trim()));
        var signature = Convert.FromBase64String(sigMatch.Groups[1].Value.Trim());

        // 3. Verify Ed25519 signature
        var ecdsa = EdDSA.Create();
        ecdsa.ImportFromPem(PublicKeyPem);
        var valid = ecdsa.VerifyData(Encoding.UTF8.GetBytes(payloadJson), signature);

        if (!valid)
            return (false, null, "Invalid signature");

        var payload = JsonSerializer.Deserialize<JsonElement>(payloadJson);

        // 4. Check expiration
        var expiresAt = payload.GetProperty("expiresAt").GetString();
        if (DateOnly.Parse(expiresAt) < DateOnly.FromDateTime(DateTime.Now))
            return (false, payload, $"License expired on {expiresAt}");

        return (true, payload, null);
    }
    catch (Exception ex)
    {
        return (false, null, ex.Message);
    }
}
```

### Best Practices — Offline

- **Embed the public key in your binary** — it's safe to distribute. Only the private key (on your server) can create valid signatures.
- **Never skip signature verification** — always verify before trusting any payload data. A tampered file with a modified `expiresAt` will fail verification.
- **Check the system clock** — offline expiration relies on the local clock. If clock tampering is a concern, consider using a monotonic counter or hardware-backed time source.
- **Feature gating** — use the `features` array to enable/disable functionality. This is more flexible than product-based gating.
- **Grace periods** — consider allowing a short grace period after expiration (e.g. 7 days) to give customers time to renew, while showing a warning.
- **File location** — let users place the `.lic` file in a known location (e.g. app directory, `~/.config/yourapp/license.lic`) and check on startup.

---

## Hybrid Approach

For apps that are sometimes online and sometimes offline, combine both:

```typescript
async function checkLicense(licenseKey: string, licFile?: string): Promise<boolean> {
  // 1. Try online verification first
  try {
    const result = await verifyOnline(licenseKey);
    if (result.valid) {
      // Cache the result locally for offline fallback
      saveCachedResult(result);
      return true;
    }
    return false; // Definitively invalid
  } catch (networkError) {
    // 2. Server unreachable — fall back to offline
    console.log("Server unreachable, trying offline verification...");
  }

  // 3. Try .lic file if provided
  if (licFile) {
    const result = verifyLicenseFile(licFile);
    if (result.valid && !result.expired) return true;
  }

  // 4. Try cached online result
  const cached = loadCachedResult(licenseKey);
  if (cached && !isCacheExpired(cached, maxAgeDays: 30)) {
    return cached.valid;
  }

  return false;
}
```

---

## Security Notes

### What's secret vs what's safe to distribute

| Asset | Secret? | Notes |
|-------|---------|-------|
| **API key** (`pk_live_...`) | Yes | Embed encrypted in your binary, or better yet use a backend proxy. If extracted, an attacker can verify arbitrary keys (but not create new ones). |
| **Public key** (`.pem`) | No | Safe to distribute. Can only verify signatures, never create them. |
| **Private key** | Yes | Never leaves the server. Stored in the database. |
| **License key** (`XXXX-XXXX-XXXX-XXXX`) | Semi | Given to the customer. Treat as confidential to prevent unauthorized sharing. |
| **`.lic` file** | No | Tamper-proof but not encrypted. The payload is base64-encoded (readable), so anyone can see customer name, features, etc. The signature prevents modification, not reading. |

### API Key Protection Strategies

For **desktop/mobile apps** where the binary ships to end users:

1. **Best: Backend proxy** — your own thin server holds the API key and proxies verification requests. The client app never has the key.
2. **Good: Encrypted embed** — encrypt the API key at build time, decrypt at runtime. Use a non-trivial decryption routine that resists static analysis.
3. **Acceptable: Obfuscation** — XOR or AES-encrypt the key with a derived key. Not bulletproof, but raises the bar significantly vs plaintext.
4. **Bad: Plaintext in source/binary** — trivially extracted with `strings`, hex editors, or decompilers. Never do this.

For **server-side apps** (your backend calling the dashboard API):
- Store the API key in environment variables or a secrets manager. This is straightforward since the key never reaches end users.

import crypto from "node:crypto";

// ---- Types ----

export interface LicensePayload {
  licenseId: string;
  customerId: string;
  customerName: string;
  productId: string;
  productName: string;
  features: string[];
  maxActivations?: number;
  issuedAt: string;
  expiresAt: string;
  metadata?: Record<string, unknown>;
}

export interface SignedLicense {
  payload: LicensePayload;
  signature: string;
}

// ---- Keypair Generation ----

export function generateKeypair(): { publicKey: string; privateKey: string } {
  const { publicKey, privateKey } = crypto.generateKeyPairSync("ed25519", {
    publicKeyEncoding: { type: "spki", format: "pem" },
    privateKeyEncoding: { type: "pkcs8", format: "pem" },
  });
  return { publicKey, privateKey };
}

// ---- Signing ----

export function signLicense(
  payload: LicensePayload,
  privateKeyPem: string,
): SignedLicense {
  const payloadJson = JSON.stringify(payload);
  const signature = crypto.sign(null, Buffer.from(payloadJson), privateKeyPem);
  return {
    payload,
    signature: signature.toString("base64"),
  };
}

// ---- Verification ----

export function verifyLicense(
  signed: SignedLicense,
  publicKeyPem: string,
): boolean {
  const payloadJson = JSON.stringify(signed.payload);
  return crypto.verify(
    null,
    Buffer.from(payloadJson),
    publicKeyPem,
    Buffer.from(signed.signature, "base64"),
  );
}

// ---- .lic File Format ----

export function toLicenseFile(signed: SignedLicense): string {
  const payloadB64 = Buffer.from(JSON.stringify(signed.payload)).toString(
    "base64",
  );
  return [
    "-----BEGIN LICENSE-----",
    payloadB64,
    "-----END LICENSE-----",
    "-----BEGIN SIGNATURE-----",
    signed.signature,
    "-----END SIGNATURE-----",
  ].join("\n");
}

export function parseLicenseFile(content: string): SignedLicense {
  const payloadMatch = content.match(
    /-----BEGIN LICENSE-----\n([\s\S]+?)\n-----END LICENSE-----/,
  );
  const sigMatch = content.match(
    /-----BEGIN SIGNATURE-----\n([\s\S]+?)\n-----END SIGNATURE-----/,
  );
  if (!payloadMatch || !sigMatch) {
    throw new Error("Invalid license file format");
  }
  const payload = JSON.parse(
    Buffer.from(payloadMatch[1].trim(), "base64").toString("utf-8"),
  );
  return { payload, signature: sigMatch[1].trim() };
}

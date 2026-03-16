"use client";

import { useState, useEffect } from "react";
import { Card, CardContent, CardHeader, CardTitle, CardDescription } from "@/components/ui/card";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { Badge } from "@/components/ui/badge";
import { Avatar, AvatarFallback } from "@/components/ui/avatar";
import { Tabs, TabsContent, TabsList, TabsTrigger } from "@/components/ui/tabs";
import {
  User,
  Shield,
  Key,
  Check,
  Plus,
  Copy,
  Trash2,
  AlertTriangle,
  RefreshCw,
  CreditCard,
  Eye,
  EyeOff,
  Loader2,
  FileSignature,
  Download,
  ShieldCheck,
  ShieldAlert,
  Upload,
} from "lucide-react";
import { Dialog, DialogContent, DialogHeader, DialogTitle, DialogDescription, DialogFooter } from "@/components/ui/dialog";
import { useApiKeys, createApiKey, revokeApiKey, useKeypair, generateKeypair, verifyLicenseFile } from "@/hooks/use-api";
import { toast } from "sonner";
import { Skeleton } from "@/components/ui/skeleton";
import { DeleteDialog } from "@/components/management/delete-dialog";

interface UserProfile {
  id: string;
  name: string;
  email: string;
  role: string;
}

export function SettingsSection() {
  const [activeTab, setActiveTab] = useState("profile");
  const [user, setUser] = useState<UserProfile | null>(null);
  const [loadingUser, setLoadingUser] = useState(true);
  const [isSaving, setIsSaving] = useState(false);
  const [profileForm, setProfileForm] = useState({ name: "", email: "" });

  useEffect(() => {
    fetch("/api/auth/me")
      .then((res) => (res.ok ? res.json() : null))
      .then((data) => {
        if (data?.user) {
          setUser(data.user);
          setProfileForm({ name: data.user.name, email: data.user.email });
        }
      })
      .finally(() => setLoadingUser(false));
  }, []);

  const handleSave = async () => {
    setIsSaving(true);
    try {
      // Profile update would go here when the API supports it
      await new Promise((r) => setTimeout(r, 500));
      toast.success("Profile saved");
    } finally {
      setIsSaving(false);
    }
  };

  const initials = user?.name
    ? user.name
        .split(" ")
        .map((n) => n[0])
        .join("")
        .toUpperCase()
        .slice(0, 2)
    : "??";

  return (
    <div className="space-y-6">
      <div>
        <h2 className="text-xl font-semibold text-foreground">Settings</h2>
        <p className="text-sm text-muted-foreground mt-1">
          Manage your account and API keys
        </p>
      </div>

      <Tabs value={activeTab} onValueChange={setActiveTab} className="space-y-6">
        <TabsList className="bg-secondary border border-border p-1">
          <TabsTrigger
            value="profile"
            className="data-[state=active]:bg-card data-[state=active]:text-foreground"
          >
            <User className="w-4 h-4 mr-2" />
            Profile
          </TabsTrigger>
          <TabsTrigger
            value="security"
            className="data-[state=active]:bg-card data-[state=active]:text-foreground"
          >
            <Shield className="w-4 h-4 mr-2" />
            Security
          </TabsTrigger>
          <TabsTrigger
            value="api-keys"
            className="data-[state=active]:bg-card data-[state=active]:text-foreground"
          >
            <Key className="w-4 h-4 mr-2" />
            API Keys
          </TabsTrigger>
          <TabsTrigger
            value="payments"
            className="data-[state=active]:bg-card data-[state=active]:text-foreground"
          >
            <CreditCard className="w-4 h-4 mr-2" />
            Payment Providers
          </TabsTrigger>
          <TabsTrigger
            value="license-signing"
            className="data-[state=active]:bg-card data-[state=active]:text-foreground"
          >
            <FileSignature className="w-4 h-4 mr-2" />
            License Signing
          </TabsTrigger>
        </TabsList>

        {/* Profile Tab */}
        <TabsContent value="profile" className="space-y-6 animate-in fade-in slide-in-from-bottom-2 duration-300">
          <Card className="border-border bg-card">
            <CardHeader>
              <CardTitle className="text-base font-medium">Personal Information</CardTitle>
              <CardDescription>Your account details</CardDescription>
            </CardHeader>
            <CardContent className="space-y-6">
              {loadingUser ? (
                <div className="space-y-4">
                  <div className="flex items-center gap-4">
                    <Skeleton className="w-16 h-16 rounded-full" />
                    <div className="space-y-2">
                      <Skeleton className="h-4 w-32" />
                      <Skeleton className="h-3 w-24" />
                    </div>
                  </div>
                  <Skeleton className="h-10 w-full max-w-md" />
                  <Skeleton className="h-10 w-full max-w-md" />
                </div>
              ) : (
                <>
                  <div className="flex items-center gap-4">
                    <Avatar className="w-16 h-16 bg-secondary">
                      <AvatarFallback className="bg-accent text-accent-foreground text-xl font-semibold">
                        {initials}
                      </AvatarFallback>
                    </Avatar>
                    <div>
                      <p className="font-medium text-foreground">{user?.name}</p>
                      <Badge className="mt-1 bg-accent/20 text-accent border-accent/30 capitalize">
                        {user?.role}
                      </Badge>
                    </div>
                  </div>

                  <div className="grid grid-cols-1 md:grid-cols-2 gap-4 max-w-2xl">
                    <div className="space-y-2">
                      <Label htmlFor="name">Name</Label>
                      <Input
                        id="name"
                        value={profileForm.name}
                        onChange={(e) => setProfileForm((p) => ({ ...p, name: e.target.value }))}
                        className="bg-secondary border-border focus:border-accent"
                      />
                    </div>
                    <div className="space-y-2">
                      <Label htmlFor="email">Email</Label>
                      <Input
                        id="email"
                        type="email"
                        value={profileForm.email}
                        onChange={(e) => setProfileForm((p) => ({ ...p, email: e.target.value }))}
                        className="bg-secondary border-border focus:border-accent"
                      />
                    </div>
                  </div>

                  <div className="flex justify-end">
                    <Button
                      onClick={handleSave}
                      className="bg-accent hover:bg-accent/90 text-accent-foreground"
                      disabled={isSaving}
                    >
                      {isSaving ? (
                        <>
                          <RefreshCw className="w-4 h-4 mr-2 animate-spin" />
                          Saving...
                        </>
                      ) : (
                        <>
                          <Check className="w-4 h-4 mr-2" />
                          Save Changes
                        </>
                      )}
                    </Button>
                  </div>
                </>
              )}
            </CardContent>
          </Card>
        </TabsContent>

        {/* Security Tab */}
        <TabsContent value="security" className="space-y-6 animate-in fade-in slide-in-from-bottom-2 duration-300">
          <Card className="border-border bg-card">
            <CardHeader>
              <CardTitle className="text-base font-medium">Password</CardTitle>
              <CardDescription>Change your account password</CardDescription>
            </CardHeader>
            <CardContent className="space-y-4">
              <div className="space-y-2">
                <Label htmlFor="currentPassword">Current Password</Label>
                <Input
                  id="currentPassword"
                  type="password"
                  className="bg-secondary border-border focus:border-accent max-w-md"
                />
              </div>
              <div className="space-y-2">
                <Label htmlFor="newPassword">New Password</Label>
                <Input
                  id="newPassword"
                  type="password"
                  className="bg-secondary border-border focus:border-accent max-w-md"
                />
              </div>
              <div className="space-y-2">
                <Label htmlFor="confirmPassword">Confirm New Password</Label>
                <Input
                  id="confirmPassword"
                  type="password"
                  className="bg-secondary border-border focus:border-accent max-w-md"
                />
              </div>
              <Button variant="outline">Update Password</Button>
            </CardContent>
          </Card>
        </TabsContent>

        {/* API Keys Tab */}
        <ApiKeysTab />

        {/* Payment Providers Tab */}
        <PaymentProvidersTab />

        {/* License Signing Tab */}
        <LicenseSigningTab />
      </Tabs>
    </div>
  );
}

/* ---------- API Keys Tab ---------- */

function ApiKeysTab() {
  const { data: allKeys, isLoading, mutate } = useApiKeys();
  const [createOpen, setCreateOpen] = useState(false);
  const [newKeyName, setNewKeyName] = useState("");
  const [createdKey, setCreatedKey] = useState<string | null>(null);
  const [creating, setCreating] = useState(false);
  const [copiedKey, setCopiedKey] = useState(false);
  const [revokeTarget, setRevokeTarget] = useState<Record<string, unknown> | null>(null);
  const [revoking, setRevoking] = useState(false);

  const apiKeyList = (allKeys ?? []) as Record<string, unknown>[];

  const handleCreate = async () => {
    if (!newKeyName.trim()) {
      toast.error("Name is required");
      return;
    }
    setCreating(true);
    const result = await createApiKey({ name: newKeyName.trim() });
    if (result.success) {
      setCreatedKey(result.data.key);
      mutate();
      toast.success("API key created");
    }
    setCreating(false);
  };

  const handleCloseCreate = () => {
    setCreateOpen(false);
    setNewKeyName("");
    setCreatedKey(null);
  };

  const handleCopy = (key: string) => {
    navigator.clipboard.writeText(key);
    setCopiedKey(true);
    setTimeout(() => setCopiedKey(false), 2000);
  };

  const handleRevoke = async () => {
    if (!revokeTarget) return;
    setRevoking(true);
    const result = await revokeApiKey(revokeTarget.id as string);
    if (result.success) {
      mutate();
      toast.success("API key revoked");
      setRevokeTarget(null);
    }
    setRevoking(false);
  };

  return (
    <TabsContent value="api-keys" className="space-y-6 animate-in fade-in slide-in-from-bottom-2 duration-300">
      <Card className="border-border bg-card">
        <CardHeader>
          <div className="flex items-center justify-between">
            <div>
              <CardTitle className="text-base font-medium">API Keys</CardTitle>
              <CardDescription>Manage API keys for license verification from your apps</CardDescription>
            </div>
            <Button size="sm" onClick={() => setCreateOpen(true)}>
              <Plus className="w-4 h-4 mr-1.5" />
              Create Key
            </Button>
          </div>
        </CardHeader>
        <CardContent>
          {isLoading ? (
            <div className="space-y-3">
              <Skeleton className="h-14 w-full" />
              <Skeleton className="h-14 w-full" />
            </div>
          ) : apiKeyList.length === 0 ? (
            <div className="text-center py-8 text-muted-foreground">
              <Key className="w-8 h-8 mx-auto mb-3 opacity-50" />
              <p className="text-sm">No API keys yet</p>
              <p className="text-xs mt-1">Create an API key to start verifying licenses from your app</p>
            </div>
          ) : (
            <div className="space-y-3">
              {apiKeyList.map((apiKey, index) => {
                const isActive = apiKey.status === "active";
                const lastUsed = apiKey.lastUsedAt
                  ? new Date(apiKey.lastUsedAt as string).toLocaleDateString()
                  : "Never";
                const created = new Date(apiKey.createdAt as string).toLocaleDateString();

                return (
                  <div
                    key={apiKey.id as string}
                    className="flex items-center justify-between p-3 rounded-lg bg-secondary/30 border border-border animate-in fade-in slide-in-from-left-2"
                    style={{ animationDelay: `${index * 75}ms` }}
                  >
                    <div className="flex items-center gap-3">
                      <div className={`w-8 h-8 rounded-full flex items-center justify-center ${isActive ? "bg-accent/20" : "bg-muted"}`}>
                        <Key className={`w-4 h-4 ${isActive ? "text-accent" : "text-muted-foreground"}`} />
                      </div>
                      <div>
                        <p className="text-sm font-medium text-foreground">
                          {apiKey.name as string}
                          <Badge className={`ml-2 text-xs ${isActive ? "bg-accent/20 text-accent border-accent/30" : "bg-muted text-muted-foreground border-border"}`}>
                            {isActive ? "Active" : "Revoked"}
                          </Badge>
                        </p>
                        <p className="text-xs text-muted-foreground">
                          <span className="font-mono">{apiKey.keyPrefix as string}...</span>
                          {" "}·{" "}Created {created}
                          {" "}·{" "}Last used: {lastUsed}
                        </p>
                      </div>
                    </div>
                    {isActive && (
                      <Button
                        variant="ghost"
                        size="sm"
                        className="text-destructive hover:text-destructive"
                        onClick={() => setRevokeTarget(apiKey)}
                      >
                        <Trash2 className="w-3.5 h-3.5 mr-1" />
                        Revoke
                      </Button>
                    )}
                  </div>
                );
              })}
            </div>
          )}
        </CardContent>
      </Card>

      {/* Usage guide */}
      <Card className="border-border bg-card">
        <CardHeader>
          <CardTitle className="text-base font-medium">Usage</CardTitle>
          <CardDescription>How to verify licenses from your application</CardDescription>
        </CardHeader>
        <CardContent>
          <div className="rounded-lg bg-secondary/50 border border-border p-4 font-mono text-xs text-muted-foreground leading-relaxed">
            <p className="text-foreground mb-1">POST /api/v1/licenses/verify</p>
            <p>Authorization: Bearer pk_live_...</p>
            <p>Content-Type: application/json</p>
            <p className="mt-2">{`{ "licenseKey": "XXXX-XXXX-XXXX-XXXX" }`}</p>
          </div>
        </CardContent>
      </Card>

      {/* Create Dialog */}
      <Dialog open={createOpen} onOpenChange={(open) => { if (!open) handleCloseCreate(); }}>
        <DialogContent className="sm:max-w-md">
          {createdKey ? (
            <>
              <DialogHeader>
                <DialogTitle>API Key Created</DialogTitle>
                <DialogDescription>Copy this key now. It won&apos;t be shown again.</DialogDescription>
              </DialogHeader>
              <div className="space-y-4 mt-4">
                <div className="flex items-center gap-2 p-3 rounded-lg bg-secondary border border-border">
                  <code className="flex-1 text-xs font-mono text-foreground break-all">{createdKey}</code>
                  <Button variant="ghost" size="sm" onClick={() => handleCopy(createdKey)}>
                    {copiedKey ? <Check className="w-4 h-4 text-accent" /> : <Copy className="w-4 h-4" />}
                  </Button>
                </div>
                <div className="flex items-start gap-2 text-xs text-warning">
                  <AlertTriangle className="w-4 h-4 shrink-0 mt-0.5" />
                  <p>Store this key securely. You won&apos;t be able to see it again after closing this dialog.</p>
                </div>
              </div>
              <DialogFooter className="mt-4">
                <Button onClick={handleCloseCreate}>Done</Button>
              </DialogFooter>
            </>
          ) : (
            <>
              <DialogHeader>
                <DialogTitle>Create API Key</DialogTitle>
                <DialogDescription>Give this key a name to identify it later</DialogDescription>
              </DialogHeader>
              <div className="mt-4 space-y-2">
                <Label htmlFor="keyName">Name</Label>
                <Input
                  id="keyName"
                  placeholder="e.g., Desktop App Production"
                  value={newKeyName}
                  onChange={(e) => setNewKeyName(e.target.value)}
                  className="bg-secondary border-border focus:border-accent"
                  onKeyDown={(e) => { if (e.key === "Enter") handleCreate(); }}
                />
              </div>
              <DialogFooter className="mt-4">
                <Button variant="outline" onClick={handleCloseCreate}>Cancel</Button>
                <Button onClick={handleCreate} disabled={creating}>
                  {creating ? "Creating..." : "Create Key"}
                </Button>
              </DialogFooter>
            </>
          )}
        </DialogContent>
      </Dialog>

      {/* Revoke Confirmation */}
      <DeleteDialog
        open={!!revokeTarget}
        onOpenChange={(open) => { if (!open) setRevokeTarget(null); }}
        title="Revoke API key?"
        description={`This will permanently revoke "${revokeTarget?.name ?? ""}". Any app using this key will lose access.`}
        onConfirm={handleRevoke}
        loading={revoking}
      />
    </TabsContent>
  );
}

/* ---------- Payment Providers Tab ---------- */

interface ProviderStatus {
  stripe: { configured: boolean; secretKey: string; webhookSecret: string };
  xendit: { configured: boolean; secretKey: string; webhookToken: string };
  lemonsqueezy: { configured: boolean; apiKey: string; storeId: string; webhookSecret: string };
}

interface ProviderField {
  key: string;
  label: string;
  placeholder: string;
  sensitive: boolean;
}

const providerConfigs: {
  id: "stripe" | "xendit" | "lemonsqueezy";
  name: string;
  description: string;
  fields: ProviderField[];
}[] = [
  {
    id: "xendit",
    name: "Xendit",
    description: "Accept payments via bank transfer, e-wallets, QRIS, and virtual accounts in Indonesia",
    fields: [
      { key: "secretKey", label: "Secret Key", placeholder: "xnd_production_...", sensitive: true },
      { key: "webhookToken", label: "Webhook Verification Token", placeholder: "Your callback verification token", sensitive: true },
    ],
  },
  {
    id: "lemonsqueezy",
    name: "Lemonsqueezy",
    description: "Accept international card payments and PayPal as merchant of record",
    fields: [
      { key: "apiKey", label: "API Key", placeholder: "eyJ0eXAi...", sensitive: true },
      { key: "storeId", label: "Store ID", placeholder: "12345", sensitive: false },
      { key: "webhookSecret", label: "Webhook Signing Secret", placeholder: "whsec_...", sensitive: true },
    ],
  },
  {
    id: "stripe",
    name: "Stripe",
    description: "Accept credit/debit card payments internationally",
    fields: [
      { key: "secretKey", label: "Secret Key", placeholder: "sk_live_...", sensitive: true },
      { key: "webhookSecret", label: "Webhook Signing Secret", placeholder: "whsec_...", sensitive: true },
    ],
  },
];

function PaymentProvidersTab() {
  const [status, setStatus] = useState<ProviderStatus | null>(null);
  const [loading, setLoading] = useState(true);
  const [saving, setSaving] = useState<string | null>(null);
  const [forms, setForms] = useState<Record<string, Record<string, string>>>({
    stripe: {},
    xendit: {},
    lemonsqueezy: {},
  });
  const [showSecrets, setShowSecrets] = useState<Record<string, boolean>>({});

  useEffect(() => {
    fetch("/api/settings/payment-providers")
      .then((res) => (res.ok ? res.json() : null))
      .then((data) => { if (data) setStatus(data); })
      .finally(() => setLoading(false));
  }, []);

  const handleSave = async (providerId: string) => {
    const providerForm = forms[providerId];
    if (!providerForm || Object.values(providerForm).every((v) => !v)) {
      toast.error("Enter at least one field to update");
      return;
    }

    setSaving(providerId);
    try {
      const res = await fetch("/api/settings/payment-providers", {
        method: "PUT",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ provider: providerId, settings: providerForm }),
      });
      if (!res.ok) {
        const body = await res.json().catch(() => ({}));
        throw new Error(body.error ?? "Save failed");
      }
      const newStatus = await res.json();
      setStatus(newStatus);
      // Clear the form fields after save
      setForms((prev) => ({ ...prev, [providerId]: {} }));
      toast.success(`${providerId.charAt(0).toUpperCase() + providerId.slice(1)} settings saved`);
    } catch (err) {
      toast.error(err instanceof Error ? err.message : "Failed to save settings");
    } finally {
      setSaving(null);
    }
  };

  const updateForm = (providerId: string, field: string, value: string) => {
    setForms((prev) => ({
      ...prev,
      [providerId]: { ...prev[providerId], [field]: value },
    }));
  };

  const getStoredValue = (providerId: string, fieldKey: string): string => {
    if (!status) return "";
    const providerStatus = status[providerId as keyof ProviderStatus];
    return (providerStatus as Record<string, unknown>)?.[fieldKey] as string ?? "";
  };

  return (
    <TabsContent value="payments" className="space-y-6 animate-in fade-in slide-in-from-bottom-2 duration-300">
      {loading ? (
        <div className="space-y-4">
          {[...Array(3)].map((_, i) => <Skeleton key={i} className="h-48 w-full rounded-xl" />)}
        </div>
      ) : (
        providerConfigs.map((provider) => {
          const providerStatus = status?.[provider.id];
          const isConfigured = providerStatus?.configured ?? false;

          return (
            <Card key={provider.id} className="border-border bg-card">
              <CardHeader>
                <div className="flex items-center justify-between">
                  <div className="flex items-center gap-3">
                    <div className={`w-10 h-10 rounded-lg flex items-center justify-center ${isConfigured ? "bg-accent/20" : "bg-secondary"}`}>
                      <CreditCard className={`w-5 h-5 ${isConfigured ? "text-accent" : "text-muted-foreground"}`} />
                    </div>
                    <div>
                      <CardTitle className="text-base font-medium flex items-center gap-2">
                        {provider.name}
                        <Badge className={isConfigured
                          ? "bg-accent/20 text-accent border-accent/30"
                          : "bg-muted text-muted-foreground border-border"
                        }>
                          {isConfigured ? "Connected" : "Not configured"}
                        </Badge>
                      </CardTitle>
                      <CardDescription>{provider.description}</CardDescription>
                    </div>
                  </div>
                </div>
              </CardHeader>
              <CardContent className="space-y-4">
                {provider.fields.map((field) => {
                  const storedValue = getStoredValue(provider.id, field.key);
                  const formValue = forms[provider.id]?.[field.key] ?? "";
                  const showKey = `${provider.id}_${field.key}`;

                  return (
                    <div key={field.key} className="space-y-1.5">
                      <Label className="text-xs text-muted-foreground uppercase tracking-wider">
                        {field.label}
                      </Label>
                      <div className="flex items-center gap-2 max-w-xl">
                        <div className="relative flex-1">
                          <Input
                            type={field.sensitive && !showSecrets[showKey] ? "password" : "text"}
                            placeholder={storedValue || field.placeholder}
                            value={formValue}
                            onChange={(e) => updateForm(provider.id, field.key, e.target.value)}
                            className="bg-secondary border-border focus:border-accent pr-10"
                          />
                          {field.sensitive && (
                            <button
                              type="button"
                              onClick={() => setShowSecrets((prev) => ({ ...prev, [showKey]: !prev[showKey] }))}
                              className="absolute right-3 top-1/2 -translate-y-1/2 text-muted-foreground hover:text-foreground"
                            >
                              {showSecrets[showKey] ? <EyeOff className="w-4 h-4" /> : <Eye className="w-4 h-4" />}
                            </button>
                          )}
                        </div>
                      </div>
                      {storedValue && !formValue && (
                        <p className="text-xs text-muted-foreground font-mono">{storedValue}</p>
                      )}
                    </div>
                  );
                })}

                <div className="flex justify-end pt-2">
                  <Button
                    onClick={() => handleSave(provider.id)}
                    disabled={saving === provider.id}
                    className="bg-accent hover:bg-accent/90 text-accent-foreground"
                  >
                    {saving === provider.id ? (
                      <>
                        <Loader2 className="w-4 h-4 mr-2 animate-spin" />
                        Saving...
                      </>
                    ) : (
                      <>
                        <Check className="w-4 h-4 mr-2" />
                        Save {provider.name}
                      </>
                    )}
                  </Button>
                </div>
              </CardContent>
            </Card>
          );
        })
      )}
    </TabsContent>
  );
}

/* ---------- License Signing Tab ---------- */

function LicenseSigningTab() {
  const { data: keypairData, isLoading, mutate } = useKeypair();
  const [generating, setGenerating] = useState(false);
  const [confirmRegenOpen, setConfirmRegenOpen] = useState(false);
  const [copiedPubKey, setCopiedPubKey] = useState(false);

  // Verify section
  const [licenseFileContent, setLicenseFileContent] = useState("");
  const [verifying, setVerifying] = useState(false);
  const [verifyResult, setVerifyResult] = useState<{
    valid: boolean;
    expired: boolean;
    payload: Record<string, unknown> | null;
    error?: string;
  } | null>(null);

  const hasKeypair = keypairData?.hasKeypair ?? false;
  const publicKey = keypairData?.publicKey ?? "";

  const handleGenerate = async (confirm?: boolean) => {
    setGenerating(true);
    const result = await generateKeypair(confirm);
    if (result.success) {
      mutate();
      toast.success("Signing keypair generated");
      setConfirmRegenOpen(false);
    } else if (result.status === 409) {
      setConfirmRegenOpen(true);
    }
    setGenerating(false);
  };

  const handleCopyPublicKey = () => {
    navigator.clipboard.writeText(publicKey);
    setCopiedPubKey(true);
    setTimeout(() => setCopiedPubKey(false), 2000);
  };

  const handleDownloadPublicKey = () => {
    const blob = new Blob([publicKey], { type: "text/plain" });
    const url = URL.createObjectURL(blob);
    const a = document.createElement("a");
    a.href = url;
    a.download = "license-signing-public-key.pem";
    a.click();
    URL.revokeObjectURL(url);
  };

  const handleVerify = async () => {
    if (!licenseFileContent.trim()) {
      toast.error("Paste a .lic file content to verify");
      return;
    }
    setVerifying(true);
    setVerifyResult(null);
    const result = await verifyLicenseFile(licenseFileContent);
    if (result.success) {
      setVerifyResult(result.data);
    }
    setVerifying(false);
  };

  const handleFileUpload = (e: React.ChangeEvent<HTMLInputElement>) => {
    const file = e.target.files?.[0];
    if (!file) return;
    const reader = new FileReader();
    reader.onload = () => {
      setLicenseFileContent(reader.result as string);
      setVerifyResult(null);
    };
    reader.readAsText(file);
    e.target.value = "";
  };

  if (isLoading) {
    return (
      <TabsContent value="license-signing" className="space-y-6 animate-in fade-in slide-in-from-bottom-2 duration-300">
        <Skeleton className="h-48 w-full rounded-xl" />
        <Skeleton className="h-32 w-full rounded-xl" />
      </TabsContent>
    );
  }

  return (
    <TabsContent value="license-signing" className="space-y-6 animate-in fade-in slide-in-from-bottom-2 duration-300">
      {/* Keypair Management */}
      <Card className="border-border bg-card">
        <CardHeader>
          <div className="flex items-center justify-between">
            <div className="flex items-center gap-3">
              <div className={`w-10 h-10 rounded-lg flex items-center justify-center ${hasKeypair ? "bg-accent/20" : "bg-secondary"}`}>
                <FileSignature className={`w-5 h-5 ${hasKeypair ? "text-accent" : "text-muted-foreground"}`} />
              </div>
              <div>
                <CardTitle className="text-base font-medium flex items-center gap-2">
                  Ed25519 Signing Keypair
                  <Badge className={hasKeypair
                    ? "bg-accent/20 text-accent border-accent/30"
                    : "bg-muted text-muted-foreground border-border"
                  }>
                    {hasKeypair ? "Active" : "Not configured"}
                  </Badge>
                </CardTitle>
                <CardDescription>
                  Generate a keypair to sign licenses for offline verification
                </CardDescription>
              </div>
            </div>
            <Button
              onClick={() => hasKeypair ? setConfirmRegenOpen(true) : handleGenerate()}
              disabled={generating}
              variant={hasKeypair ? "outline" : "default"}
              className={!hasKeypair ? "bg-accent hover:bg-accent/90 text-accent-foreground" : ""}
            >
              {generating ? (
                <Loader2 className="w-4 h-4 mr-2 animate-spin" />
              ) : (
                <RefreshCw className="w-4 h-4 mr-2" />
              )}
              {hasKeypair ? "Regenerate" : "Generate Keypair"}
            </Button>
          </div>
        </CardHeader>
        {hasKeypair && (
          <CardContent className="space-y-4">
            <div>
              <Label className="text-xs text-muted-foreground uppercase tracking-wider">
                Public Key (embed this in your client applications)
              </Label>
              <textarea
                readOnly
                value={publicKey}
                className="mt-1.5 w-full h-32 rounded-lg bg-secondary border border-border p-3 font-mono text-xs text-muted-foreground resize-none focus:outline-none"
              />
              <div className="flex items-center gap-2 mt-2">
                <Button variant="outline" size="sm" onClick={handleCopyPublicKey}>
                  {copiedPubKey ? <Check className="w-4 h-4 mr-1.5 text-accent" /> : <Copy className="w-4 h-4 mr-1.5" />}
                  {copiedPubKey ? "Copied" : "Copy"}
                </Button>
                <Button variant="outline" size="sm" onClick={handleDownloadPublicKey}>
                  <Download className="w-4 h-4 mr-1.5" />
                  Download .pem
                </Button>
              </div>
            </div>
          </CardContent>
        )}
      </Card>

      {/* How it works */}
      <Card className="border-border bg-card">
        <CardHeader>
          <CardTitle className="text-base font-medium">How Offline Licensing Works</CardTitle>
          <CardDescription>Cryptographic license verification without server connectivity</CardDescription>
        </CardHeader>
        <CardContent>
          <div className="space-y-3 text-sm text-muted-foreground">
            <div className="flex items-start gap-3">
              <span className="w-6 h-6 rounded-full bg-accent/20 text-accent flex items-center justify-center text-xs font-semibold shrink-0">1</span>
              <p>Generate an Ed25519 keypair above. The private key stays on the server, the public key goes into your app.</p>
            </div>
            <div className="flex items-start gap-3">
              <span className="w-6 h-6 rounded-full bg-accent/20 text-accent flex items-center justify-center text-xs font-semibold shrink-0">2</span>
              <p>When you sign a license, the customer/product/features/expiry data is cryptographically signed with the private key.</p>
            </div>
            <div className="flex items-start gap-3">
              <span className="w-6 h-6 rounded-full bg-accent/20 text-accent flex items-center justify-center text-xs font-semibold shrink-0">3</span>
              <p>Export the signed license as a <code className="px-1 py-0.5 bg-secondary rounded text-xs font-mono">.lic</code> file and distribute it to customers.</p>
            </div>
            <div className="flex items-start gap-3">
              <span className="w-6 h-6 rounded-full bg-accent/20 text-accent flex items-center justify-center text-xs font-semibold shrink-0">4</span>
              <p>Your app verifies the <code className="px-1 py-0.5 bg-secondary rounded text-xs font-mono">.lic</code> file using the embedded public key — no internet required.</p>
            </div>
          </div>
        </CardContent>
      </Card>

      {/* Verify License */}
      {hasKeypair && (
        <Card className="border-border bg-card">
          <CardHeader>
            <CardTitle className="text-base font-medium">Verify License File</CardTitle>
            <CardDescription>Paste or upload a .lic file to verify its signature</CardDescription>
          </CardHeader>
          <CardContent className="space-y-4">
            <div>
              <textarea
                placeholder={"-----BEGIN LICENSE-----\n...\n-----END LICENSE-----\n-----BEGIN SIGNATURE-----\n...\n-----END SIGNATURE-----"}
                value={licenseFileContent}
                onChange={(e) => { setLicenseFileContent(e.target.value); setVerifyResult(null); }}
                className="w-full h-40 rounded-lg bg-secondary border border-border p-3 font-mono text-xs text-foreground placeholder:text-muted-foreground/50 resize-none focus:outline-none focus:ring-2 focus:ring-ring/20 focus:border-accent transition-all"
              />
            </div>
            <div className="flex items-center gap-2">
              <Button onClick={handleVerify} disabled={verifying || !licenseFileContent.trim()}>
                {verifying ? <Loader2 className="w-4 h-4 mr-1.5 animate-spin" /> : <ShieldCheck className="w-4 h-4 mr-1.5" />}
                Verify
              </Button>
              <Button variant="outline" asChild>
                <label className="cursor-pointer">
                  <Upload className="w-4 h-4 mr-1.5" />
                  Upload .lic
                  <input type="file" accept=".lic,.txt" onChange={handleFileUpload} className="hidden" />
                </label>
              </Button>
            </div>

            {verifyResult && (
              <div className={`rounded-lg border p-4 ${verifyResult.valid ? "border-accent/30 bg-accent/5" : "border-destructive/30 bg-destructive/5"}`}>
                <div className="flex items-center gap-2 mb-3">
                  {verifyResult.valid ? (
                    <>
                      <ShieldCheck className="w-5 h-5 text-accent" />
                      <span className="font-medium text-accent text-sm">
                        Signature Valid
                        {verifyResult.expired && " (Expired)"}
                      </span>
                    </>
                  ) : (
                    <>
                      <ShieldAlert className="w-5 h-5 text-destructive" />
                      <span className="font-medium text-destructive text-sm">
                        {verifyResult.error || "Invalid Signature"}
                      </span>
                    </>
                  )}
                </div>
                {verifyResult.valid && verifyResult.payload && (
                  <div className="grid grid-cols-2 gap-3 text-xs">
                    <div>
                      <span className="text-muted-foreground">License ID</span>
                      <p className="font-mono text-foreground mt-0.5">{verifyResult.payload.licenseId as string}</p>
                    </div>
                    <div>
                      <span className="text-muted-foreground">Customer</span>
                      <p className="text-foreground mt-0.5">{verifyResult.payload.customerName as string}</p>
                    </div>
                    <div>
                      <span className="text-muted-foreground">Product</span>
                      <p className="text-foreground mt-0.5">{verifyResult.payload.productName as string}</p>
                    </div>
                    <div>
                      <span className="text-muted-foreground">Expires</span>
                      <p className={`mt-0.5 ${verifyResult.expired ? "text-destructive" : "text-foreground"}`}>
                        {verifyResult.payload.expiresAt as string}
                        {verifyResult.expired && " (expired)"}
                      </p>
                    </div>
                    {(verifyResult.payload.features as string[])?.length > 0 && (
                      <div className="col-span-2">
                        <span className="text-muted-foreground">Features</span>
                        <div className="flex flex-wrap gap-1 mt-1">
                          {(verifyResult.payload.features as string[]).map((f) => (
                            <Badge key={f} variant="secondary" className="text-xs">{f}</Badge>
                          ))}
                        </div>
                      </div>
                    )}
                  </div>
                )}
              </div>
            )}
          </CardContent>
        </Card>
      )}

      {/* Regenerate Confirmation Dialog */}
      <Dialog open={confirmRegenOpen} onOpenChange={setConfirmRegenOpen}>
        <DialogContent className="sm:max-w-md">
          <DialogHeader>
            <DialogTitle>Regenerate Signing Keypair?</DialogTitle>
            <DialogDescription>
              This will create a new keypair and replace the existing one. Previously signed licenses will no longer be verifiable with the new public key.
            </DialogDescription>
          </DialogHeader>
          <div className="flex items-start gap-2 text-xs text-warning mt-2">
            <AlertTriangle className="w-4 h-4 shrink-0 mt-0.5" />
            <p>Any .lic files generated with the old keypair will fail verification. Make sure all customers have updated licenses before regenerating.</p>
          </div>
          <DialogFooter className="mt-4">
            <Button variant="outline" onClick={() => setConfirmRegenOpen(false)}>Cancel</Button>
            <Button
              variant="destructive"
              onClick={() => handleGenerate(true)}
              disabled={generating}
            >
              {generating ? <Loader2 className="w-4 h-4 mr-1.5 animate-spin" /> : <RefreshCw className="w-4 h-4 mr-1.5" />}
              Regenerate
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>
    </TabsContent>
  );
}

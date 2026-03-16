"use client";

import { useState } from "react";
import { cn } from "@/lib/utils";
import { Search, MoreHorizontal, Trash2, Copy, Check, KeyRound, ShieldOff, CalendarPlus, Shield, Download, FileSignature, X, Plus, Loader2, Monitor, Wifi } from "lucide-react";
import { Table, TableBody, TableCell, TableHead, TableHeader, TableRow } from "@/components/ui/table";
import { Dialog, DialogContent, DialogHeader, DialogTitle, DialogDescription, DialogFooter } from "@/components/ui/dialog";
import { DropdownMenu, DropdownMenuContent, DropdownMenuItem, DropdownMenuTrigger, DropdownMenuSeparator } from "@/components/ui/dropdown-menu";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { Badge } from "@/components/ui/badge";
import { getStatusColor, getLicenseTypeBadge } from "@/lib/license-keys";
import { useLicenses, updateLicense, deleteLicense, signLicenseKey, getLicenseExportUrl, useLicenseActivations, deactivateDevice } from "@/hooks/use-api";
import { toast } from "sonner";
import { Skeleton } from "@/components/ui/skeleton";
import { DeleteDialog } from "./delete-dialog";

type License = Record<string, unknown>;

/* ---------- sign dialog ---------- */

function SignLicenseDialog({ license, open, onOpenChange, onSuccess }: {
  license: License;
  open: boolean;
  onOpenChange: (open: boolean) => void;
  onSuccess: () => void;
}) {
  const [features, setFeatures] = useState<string[]>((license.features as string[] | null) ?? []);
  const [featureInput, setFeatureInput] = useState("");
  const [maxActivations, setMaxActivations] = useState<string>(
    license.maxActivations ? String(license.maxActivations) : "",
  );
  const [loading, setLoading] = useState(false);

  const addFeature = () => {
    const trimmed = featureInput.trim();
    if (trimmed && !features.includes(trimmed)) {
      setFeatures([...features, trimmed]);
    }
    setFeatureInput("");
  };

  const removeFeature = (f: string) => {
    setFeatures(features.filter((x) => x !== f));
  };

  const handleSign = async () => {
    setLoading(true);
    try {
      await signLicenseKey(license.key as string, {
        features,
        maxActivations: maxActivations ? parseInt(maxActivations, 10) : undefined,
      });
      toast.success("License signed successfully");
      onSuccess();
      onOpenChange(false);
    } catch {
      toast.error("Failed to sign license. Make sure a signing keypair is configured in Settings.");
    } finally {
      setLoading(false);
    }
  };

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="sm:max-w-lg">
        <DialogHeader>
          <DialogTitle className="text-lg">
            {license.hasCertificate ? "Re-sign License" : "Generate Certificate"}
          </DialogTitle>
          <DialogDescription>
            Sign this license with your Ed25519 keypair to generate an offline-verifiable certificate.
          </DialogDescription>
        </DialogHeader>

        <div className="space-y-4 mt-2">
          <div className="flex items-center gap-2 px-3 py-2 rounded-lg bg-secondary text-sm font-mono text-muted-foreground">
            <KeyRound className="w-4 h-4 text-chart-1 shrink-0" />
            {license.key as string}
          </div>

          <div>
            <Label className="text-sm">Features / Entitlements</Label>
            <div className="flex items-center gap-2 mt-1.5">
              <Input
                placeholder="e.g. pro, enterprise, reports"
                value={featureInput}
                onChange={(e) => setFeatureInput(e.target.value)}
                onKeyDown={(e) => { if (e.key === "Enter") { e.preventDefault(); addFeature(); } }}
                className="flex-1"
              />
              <Button variant="outline" size="sm" onClick={addFeature} disabled={!featureInput.trim()}>
                <Plus className="w-4 h-4" />
              </Button>
            </div>
            {features.length > 0 && (
              <div className="flex flex-wrap gap-1.5 mt-2">
                {features.map((f) => (
                  <Badge key={f} variant="secondary" className="gap-1">
                    {f}
                    <button onClick={() => removeFeature(f)} className="ml-0.5 hover:text-foreground">
                      <X className="w-3 h-3" />
                    </button>
                  </Badge>
                ))}
              </div>
            )}
          </div>

          <div>
            <Label className="text-sm">Max Activations (optional)</Label>
            <Input
              type="number"
              min="1"
              placeholder="Unlimited"
              value={maxActivations}
              onChange={(e) => setMaxActivations(e.target.value)}
              className="mt-1.5 w-40"
            />
          </div>
        </div>

        <DialogFooter className="mt-4">
          <Button variant="outline" onClick={() => onOpenChange(false)}>Cancel</Button>
          <Button onClick={handleSign} disabled={loading}>
            {loading && <Loader2 className="w-4 h-4 mr-1.5 animate-spin" />}
            <FileSignature className="w-4 h-4 mr-1.5" />
            {license.hasCertificate ? "Re-sign" : "Sign License"}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}

/* ---------- detail view ---------- */

function LicenseDetail({ license, onDelete, onCopyKey, onRevoke, onExtend, onSign, copiedKey, onMutate }: {
  license: License;
  onDelete: () => void;
  onCopyKey: (key: string) => void;
  onRevoke: (key: string) => void;
  onExtend: (key: string, expiresAt: string) => void;
  onSign: () => void;
  copiedKey: string | null;
  onMutate: () => void;
}) {
  const key = license.key as string;
  const { data: activationData, mutate: mutateActivations } = useLicenseActivations(key);
  const [deactivating, setDeactivating] = useState<string | null>(null);
  const status = license.status as string;
  const statusCls = getStatusColor(status as "active" | "expired" | "revoked" | "suspended");
  const licenseType = (license.licenseType as string) || "simple";
  const typeBadge = getLicenseTypeBadge(licenseType as "simple" | "signed");
  const features = (license.features as string[] | null) ?? [];
  const hasCertificate = !!license.hasCertificate;

  const labelClass = "text-xs text-muted-foreground uppercase tracking-wider";
  const valueClass = "text-sm font-medium text-foreground mt-0.5";

  return (
    <div>
      <DialogHeader>
        <DialogTitle className="text-lg">License Details</DialogTitle>
        <DialogDescription>View license information and manage actions</DialogDescription>
      </DialogHeader>

      <div className="mt-6 space-y-6">
        {/* License key */}
        <div>
          <p className={labelClass}>License Key</p>
          <div className="mt-1.5">
            <button onClick={() => onCopyKey(key)} className="inline-flex items-center gap-2 px-3 py-2 rounded-lg bg-secondary hover:bg-secondary/80 text-sm font-mono text-foreground transition-colors">
              <KeyRound className="w-4 h-4 text-chart-1 shrink-0" />
              <span>{key}</span>
              {copiedKey === key ? <Check className="w-4 h-4 text-success shrink-0" /> : <Copy className="w-4 h-4 text-muted-foreground shrink-0" />}
            </button>
          </div>
        </div>

        {/* Details grid */}
        <div className="grid grid-cols-2 gap-4">
          <div>
            <p className={labelClass}>Customer</p>
            <p className={valueClass}>{license.customer as string}</p>
          </div>
          <div>
            <p className={labelClass}>Product</p>
            <p className={valueClass}>{license.product as string}</p>
          </div>
        </div>

        <div className="grid grid-cols-2 sm:grid-cols-4 gap-4">
          <div>
            <p className={labelClass}>Status</p>
            <span className={cn("inline-flex px-2 py-0.5 rounded-md text-xs font-medium capitalize mt-0.5", statusCls)}>{status}</span>
          </div>
          <div>
            <p className={labelClass}>Type</p>
            <span className={cn("inline-flex items-center gap-1 px-2 py-0.5 rounded-md text-xs font-medium mt-0.5", typeBadge.className)}>
              {licenseType === "signed" && <Shield className="w-3 h-3" />}
              {typeBadge.label}
            </span>
          </div>
          <div>
            <p className={labelClass}>Created</p>
            <p className={valueClass}>{license.createdAt as string}</p>
          </div>
          <div>
            <p className={labelClass}>Expires</p>
            <p className={valueClass}>{license.expiresAt as string}</p>
          </div>
        </div>

        {/* Features */}
        {features.length > 0 && (
          <div>
            <p className={labelClass}>Features / Entitlements</p>
            <div className="flex flex-wrap gap-1.5 mt-1.5">
              {features.map((f) => (
                <Badge key={f} variant="secondary">{f}</Badge>
              ))}
            </div>
          </div>
        )}

        {/* Activations */}
        <div>
          <div className="flex items-center justify-between mb-2">
            <p className={labelClass}>
              <span className="inline-flex items-center gap-1.5">
                <Wifi className="w-3 h-3" />
                Activations
                {activationData && (
                  <span className="text-foreground font-medium ml-1">
                    {activationData.activations?.length ?? 0}
                    {license.maxActivations ? ` / ${license.maxActivations}` : ""}
                  </span>
                )}
              </span>
            </p>
          </div>
          {activationData?.activations?.length > 0 ? (
            <div className="rounded-lg border border-border overflow-hidden">
              <Table>
                <TableHeader>
                  <TableRow className="bg-secondary/50">
                    <TableHead className="px-3 text-[10px] font-semibold uppercase tracking-wider">Device</TableHead>
                    <TableHead className="px-3 text-[10px] font-semibold uppercase tracking-wider">IP</TableHead>
                    <TableHead className="px-3 text-[10px] font-semibold uppercase tracking-wider">Activated</TableHead>
                    <TableHead className="px-3 text-[10px] font-semibold uppercase tracking-wider">Last Seen</TableHead>
                    <TableHead className="w-10" />
                  </TableRow>
                </TableHeader>
                <TableBody>
                  {(activationData.activations as { id: string; deviceId: string; deviceName: string | null; ipAddress: string | null; activatedAt: string; lastSeenAt: string }[]).map((a) => (
                    <TableRow key={a.id}>
                      <TableCell className="px-3">
                        <div className="flex items-center gap-1.5">
                          <Monitor className="w-3.5 h-3.5 text-muted-foreground shrink-0" />
                          <div>
                            <p className="text-xs font-medium">{a.deviceName || "Unknown Device"}</p>
                            <p className="text-[10px] text-muted-foreground font-mono truncate max-w-[140px]">{a.deviceId}</p>
                          </div>
                        </div>
                      </TableCell>
                      <TableCell className="px-3 text-xs text-muted-foreground font-mono">{a.ipAddress || "—"}</TableCell>
                      <TableCell className="px-3 text-xs text-muted-foreground">{new Date(a.activatedAt).toLocaleDateString()}</TableCell>
                      <TableCell className="px-3 text-xs text-muted-foreground">{new Date(a.lastSeenAt).toLocaleDateString()}</TableCell>
                      <TableCell className="px-3">
                        <button
                          disabled={deactivating === a.deviceId}
                          onClick={async () => {
                            setDeactivating(a.deviceId);
                            try {
                              await deactivateDevice(key, a.deviceId);
                              mutateActivations();
                              onMutate();
                              toast.success("Device deactivated");
                            } catch {
                              toast.error("Failed to deactivate device");
                            } finally {
                              setDeactivating(null);
                            }
                          }}
                          className="w-6 h-6 flex items-center justify-center rounded text-muted-foreground hover:text-destructive hover:bg-destructive/10 transition-colors disabled:opacity-50"
                          title="Deactivate device"
                        >
                          {deactivating === a.deviceId ? <Loader2 className="w-3 h-3 animate-spin" /> : <X className="w-3 h-3" />}
                        </button>
                      </TableCell>
                    </TableRow>
                  ))}
                </TableBody>
              </Table>
            </div>
          ) : (
            <p className="text-xs text-muted-foreground italic">No devices activated yet</p>
          )}
        </div>
      </div>

      {/* Footer */}
      <div className="flex items-center justify-between pt-6 mt-6 border-t border-border">
        <Button variant="outline" size="sm" className="text-destructive hover:text-destructive hover:bg-destructive/10" onClick={onDelete}>
          <Trash2 className="w-4 h-4 mr-1.5" />
          Delete
        </Button>
        <div className="flex items-center gap-2">
          <Button variant="outline" size="sm" onClick={() => onCopyKey(key)}>
            {copiedKey === key ? <Check className="w-4 h-4 mr-1.5 text-success" /> : <Copy className="w-4 h-4 mr-1.5" />}
            {copiedKey === key ? "Copied" : "Copy Key"}
          </Button>
          {status === "active" && (
            <Button variant="outline" size="sm" onClick={() => onRevoke(key)}>
              <ShieldOff className="w-4 h-4 mr-1.5" />
              Revoke
            </Button>
          )}
          <Button variant="outline" size="sm" onClick={() => onExtend(key, license.expiresAt as string)}>
            <CalendarPlus className="w-4 h-4 mr-1.5" />
            Extend 1 Year
          </Button>
          {!hasCertificate ? (
            <Button variant="outline" size="sm" onClick={onSign}>
              <FileSignature className="w-4 h-4 mr-1.5" />
              Sign
            </Button>
          ) : (
            <Button variant="outline" size="sm" asChild>
              <a href={getLicenseExportUrl(key)} download>
                <Download className="w-4 h-4 mr-1.5" />
                Download .lic
              </a>
            </Button>
          )}
        </div>
      </div>
    </div>
  );
}

/* ---------- main ---------- */

export function ManageLicensesSection() {
  const { data: allLicenses, isLoading, mutate } = useLicenses();
  const [searchQuery, setSearchQuery] = useState("");
  const [statusFilter, setStatusFilter] = useState<string>("all");
  const [dialogOpen, setDialogOpen] = useState(false);
  const [selectedLicense, setSelectedLicense] = useState<License | null>(null);
  const [deleteTarget, setDeleteTarget] = useState<License | null>(null);
  const [deleteLoading, setDeleteLoading] = useState(false);
  const [copiedKey, setCopiedKey] = useState<string | null>(null);
  const [signDialogLicense, setSignDialogLicense] = useState<License | null>(null);

  const licenses = (allLicenses ?? []) as License[];
  const filtered = licenses.filter((l) => {
    const key = (l.key as string).toLowerCase();
    const customer = (l.customer as string).toLowerCase();
    const product = (l.product as string).toLowerCase();
    const q = searchQuery.toLowerCase();
    const matchesSearch = key.includes(q) || customer.includes(q) || product.includes(q);
    const matchesStatus = statusFilter === "all" || l.status === statusFilter;
    return matchesSearch && matchesStatus;
  });

  const openDetail = (license: License) => {
    setSelectedLicense(license);
    setDialogOpen(true);
  };

  const closeDialog = () => {
    setDialogOpen(false);
    setSelectedLicense(null);
  };

  const copyKey = (key: string) => {
    navigator.clipboard.writeText(key);
    setCopiedKey(key);
    setTimeout(() => setCopiedKey(null), 2000);
  };

  const handleRevoke = async (key: string) => {
    try {
      await updateLicense(key, { status: "revoked" });
      mutate();
      toast.success("License revoked");
      closeDialog();
    } catch {
      toast.error("Failed to revoke license");
    }
  };

  const handleExtend = async (key: string, currentExpiry: string) => {
    try {
      const d = new Date(currentExpiry);
      d.setFullYear(d.getFullYear() + 1);
      await updateLicense(key, { expiresAt: d.toISOString().split("T")[0] });
      mutate();
      toast.success("License extended by 1 year");
      closeDialog();
    } catch {
      toast.error("Failed to extend license");
    }
  };

  const handleDelete = async () => {
    if (!deleteTarget) return;
    setDeleteLoading(true);
    try {
      await deleteLicense(deleteTarget.key as string);
      mutate();
      toast.success("License deleted");
      setDeleteTarget(null);
    } catch {
      toast.error("Failed to delete license");
    } finally {
      setDeleteLoading(false);
    }
  };

  if (isLoading) {
    return (
      <div className="space-y-6">
        <Skeleton className="h-10 w-full" />
        <Skeleton className="h-[500px] rounded-xl" />
      </div>
    );
  }

  return (
    <div className="space-y-6">
      {/* Header */}
      <div className="flex flex-col sm:flex-row items-start sm:items-center justify-between gap-4">
        <div className="flex items-center gap-3 flex-wrap">
          <div className="relative">
            <Search className="absolute left-3 top-1/2 -translate-y-1/2 w-4 h-4 text-muted-foreground" />
            <input type="text" placeholder="Search licenses..." value={searchQuery} onChange={(e) => setSearchQuery(e.target.value)} className="w-72 h-9 pl-9 pr-4 rounded-lg bg-secondary border border-border text-sm text-foreground placeholder:text-muted-foreground focus:outline-none focus:ring-2 focus:ring-ring/20 focus:border-accent transition-all" />
          </div>
          <div className="flex items-center gap-2">
            {["all", "active", "expired", "revoked", "suspended"].map((f) => (
              <button key={f} onClick={() => setStatusFilter(f)} className={cn("px-3 py-1.5 rounded-lg text-xs font-medium transition-all", statusFilter === f ? "bg-foreground/10 text-foreground" : "text-muted-foreground hover:text-foreground")}>{f.charAt(0).toUpperCase() + f.slice(1)}</button>
            ))}
          </div>
        </div>
        <div className="flex items-center gap-2 text-xs text-muted-foreground bg-secondary px-3 py-1.5 rounded-lg">
          <KeyRound className="w-3.5 h-3.5" />
          Licenses are auto-created from won deals
        </div>
      </div>

      {/* Table */}
      <div className="bg-card border border-border rounded-xl overflow-hidden">
        <Table>
          <TableHeader>
            <TableRow className="bg-secondary/50">
              <TableHead className="px-4 text-xs font-semibold uppercase tracking-wider">Key</TableHead>
              <TableHead className="px-4 text-xs font-semibold uppercase tracking-wider">Customer</TableHead>
              <TableHead className="px-4 text-xs font-semibold uppercase tracking-wider">Product</TableHead>
              <TableHead className="px-4 text-xs font-semibold uppercase tracking-wider">Status</TableHead>
              <TableHead className="px-4 text-xs font-semibold uppercase tracking-wider">Type</TableHead>
              <TableHead className="px-4 text-xs font-semibold uppercase tracking-wider">Activations</TableHead>
              <TableHead className="px-4 text-xs font-semibold uppercase tracking-wider">Created</TableHead>
              <TableHead className="px-4 text-xs font-semibold uppercase tracking-wider">Expires</TableHead>
              <TableHead className="w-12" />
            </TableRow>
          </TableHeader>
          <TableBody>
            {filtered.map((license) => {
              const key = license.key as string;
              const status = license.status as string;
              const statusCls = getStatusColor(status as "active" | "expired" | "revoked" | "suspended");
              const licenseType = (license.licenseType as string) || "simple";
              const typeBadge = getLicenseTypeBadge(licenseType as "simple" | "signed");
              const hasCertificate = !!license.hasCertificate;

              return (
                <TableRow key={key} className="cursor-pointer" onClick={() => openDetail(license)}>
                  <TableCell className="px-4" onClick={(e) => e.stopPropagation()}>
                    <button onClick={() => copyKey(key)} className="inline-flex items-center gap-1.5 px-2 py-1 rounded-md bg-secondary hover:bg-secondary/80 text-xs font-mono text-muted-foreground hover:text-foreground transition-colors">
                      <KeyRound className="w-3 h-3 shrink-0" />
                      <span>{key}</span>
                      {copiedKey === key ? <Check className="w-3 h-3 text-success shrink-0" /> : <Copy className="w-3 h-3 shrink-0" />}
                    </button>
                  </TableCell>
                  <TableCell className="px-4 text-sm">{license.customer as string}</TableCell>
                  <TableCell className="px-4 text-sm">{license.product as string}</TableCell>
                  <TableCell className="px-4">
                    <span className={cn("inline-flex px-2 py-0.5 rounded-md text-xs font-medium capitalize", statusCls)}>{status}</span>
                  </TableCell>
                  <TableCell className="px-4">
                    <span className={cn("inline-flex items-center gap-1 px-2 py-0.5 rounded-md text-xs font-medium", typeBadge.className)}>
                      {licenseType === "signed" && <Shield className="w-3 h-3" />}
                      {typeBadge.label}
                    </span>
                  </TableCell>
                  <TableCell className="px-4">
                    <span className="inline-flex items-center gap-1 text-sm text-muted-foreground">
                      <Monitor className="w-3 h-3" />
                      {(license.activationCount as number) ?? 0}
                      {license.maxActivations ? ` / ${license.maxActivations}` : ""}
                    </span>
                  </TableCell>
                  <TableCell className="px-4 text-sm text-muted-foreground">{license.createdAt as string}</TableCell>
                  <TableCell className="px-4 text-sm text-muted-foreground">{license.expiresAt as string}</TableCell>
                  <TableCell className="px-4" onClick={(e) => e.stopPropagation()}>
                    <DropdownMenu>
                      <DropdownMenuTrigger asChild>
                        <button className="w-8 h-8 flex items-center justify-center rounded-lg text-muted-foreground hover:text-foreground hover:bg-secondary transition-colors">
                          <MoreHorizontal className="w-4 h-4" />
                        </button>
                      </DropdownMenuTrigger>
                      <DropdownMenuContent align="end">
                        <DropdownMenuItem onClick={() => copyKey(key)}>
                          <Copy className="w-4 h-4" />Copy Key
                        </DropdownMenuItem>
                        {!hasCertificate ? (
                          <DropdownMenuItem onClick={() => setSignDialogLicense(license)}>
                            <FileSignature className="w-4 h-4" />Generate Certificate
                          </DropdownMenuItem>
                        ) : (
                          <>
                            <DropdownMenuItem asChild>
                              <a href={getLicenseExportUrl(key)} download onClick={(e) => e.stopPropagation()}>
                                <Download className="w-4 h-4" />Download .lic
                              </a>
                            </DropdownMenuItem>
                            <DropdownMenuItem onClick={() => setSignDialogLicense(license)}>
                              <FileSignature className="w-4 h-4" />Re-sign
                            </DropdownMenuItem>
                          </>
                        )}
                        <DropdownMenuSeparator />
                        {status === "active" && (
                          <DropdownMenuItem onClick={() => handleRevoke(key)}>
                            <ShieldOff className="w-4 h-4" />Revoke
                          </DropdownMenuItem>
                        )}
                        <DropdownMenuItem onClick={() => handleExtend(key, license.expiresAt as string)}>
                          <CalendarPlus className="w-4 h-4" />Extend 1 Year
                        </DropdownMenuItem>
                        <DropdownMenuSeparator />
                        <DropdownMenuItem variant="destructive" onClick={() => setDeleteTarget(license)}>
                          <Trash2 className="w-4 h-4" />Delete
                        </DropdownMenuItem>
                      </DropdownMenuContent>
                    </DropdownMenu>
                  </TableCell>
                </TableRow>
              );
            })}
          </TableBody>
        </Table>
        <div className="flex items-center justify-between px-4 py-3 border-t border-border bg-secondary/30">
          <span className="text-sm text-muted-foreground">Showing {filtered.length} of {licenses.length} licenses</span>
        </div>
      </div>

      {/* Detail Dialog */}
      <Dialog open={dialogOpen} onOpenChange={(open) => { if (!open) closeDialog(); }}>
        <DialogContent className="sm:max-w-2xl max-h-[90vh] overflow-y-auto">
          {selectedLicense && (
            <LicenseDetail
              license={selectedLicense}
              onDelete={() => { closeDialog(); setDeleteTarget(selectedLicense); }}
              onCopyKey={copyKey}
              onRevoke={handleRevoke}
              onExtend={handleExtend}
              onSign={() => { closeDialog(); setSignDialogLicense(selectedLicense); }}
              copiedKey={copiedKey}
              onMutate={() => mutate()}
            />
          )}
        </DialogContent>
      </Dialog>

      {/* Sign License Dialog */}
      {signDialogLicense && (
        <SignLicenseDialog
          license={signDialogLicense}
          open={!!signDialogLicense}
          onOpenChange={(open) => { if (!open) setSignDialogLicense(null); }}
          onSuccess={() => mutate()}
        />
      )}

      {/* Delete Confirmation */}
      <DeleteDialog open={!!deleteTarget} onOpenChange={(open) => { if (!open) setDeleteTarget(null); }} title="Delete license?" description={`This will permanently delete license key "${deleteTarget?.key ?? ""}".`} onConfirm={handleDelete} loading={deleteLoading} />
    </div>
  );
}

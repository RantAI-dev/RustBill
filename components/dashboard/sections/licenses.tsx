"use client";

import { useState, useEffect } from "react";
import { cn } from "@/lib/utils";
import {
  Search,
  KeyRound,
  CheckCircle2,
  Clock,
  XCircle,
  AlertTriangle,
  ChevronLeft,
  ChevronRight,
  MoreHorizontal,
  Trash2,
  Copy,
  Check,
  Shield,
  ShieldOff,
  CalendarPlus,
  Download,
  FileSignature,
  X,
  Plus,
  Loader2,
  Monitor,
  Wifi,
} from "lucide-react";
import { Dialog, DialogContent, DialogHeader, DialogTitle, DialogDescription, DialogFooter } from "@/components/ui/dialog";
import { DropdownMenu, DropdownMenuContent, DropdownMenuItem, DropdownMenuTrigger, DropdownMenuSeparator } from "@/components/ui/dropdown-menu";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { Badge } from "@/components/ui/badge";
import { Table, TableBody, TableCell, TableHead, TableHeader, TableRow } from "@/components/ui/table";
import { getStatusColor, getLicenseTypeBadge } from "@/lib/license-keys";
import type { License, LicenseStatus } from "@/lib/license-keys";
import { MetricCard } from "@/components/dashboard/metric-card";
import { useLicenses, updateLicense, deleteLicense, signLicenseKey, getLicenseExportUrl, useLicenseActivations, deactivateDevice } from "@/hooks/use-api";
import { Skeleton } from "@/components/ui/skeleton";
import { DeleteDialog } from "@/components/management/delete-dialog";
import { toast } from "sonner";

const statusIcons: Record<LicenseStatus, React.ElementType> = {
  active: CheckCircle2,
  expired: Clock,
  revoked: XCircle,
  suspended: AlertTriangle,
};

const PER_PAGE = 20;

/* ---------- sign dialog ---------- */

function SignLicenseDialog({ license, open, onOpenChange, onSuccess }: {
  license: License;
  open: boolean;
  onOpenChange: (open: boolean) => void;
  onSuccess: () => void;
}) {
  const [features, setFeatures] = useState<string[]>(license.features ?? []);
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
    const result = await signLicenseKey(license.key, {
      features,
      maxActivations: maxActivations ? parseInt(maxActivations, 10) : undefined,
    });
    if (result.success) {
      toast.success("License signed successfully");
      onSuccess();
      onOpenChange(false);
    }
    setLoading(false);
  };

  const hasCertificate = !!license.hasCertificate;

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="sm:max-w-lg">
        <DialogHeader>
          <DialogTitle className="text-lg">
            {hasCertificate ? "Re-sign License" : "Generate Certificate"}
          </DialogTitle>
          <DialogDescription>
            Sign this license with your Ed25519 keypair to generate an offline-verifiable certificate.
          </DialogDescription>
        </DialogHeader>

        <div className="space-y-4 mt-2">
          <div className="flex items-center gap-2 px-3 py-2 rounded-lg bg-secondary text-sm font-mono text-muted-foreground">
            <KeyRound className="w-4 h-4 text-chart-1 shrink-0" />
            {license.key}
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
            {hasCertificate ? "Re-sign" : "Sign License"}
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
  const key = license.key;
  const { data: activationData, mutate: mutateActivations } = useLicenseActivations(key);
  const [deactivating, setDeactivating] = useState<string | null>(null);
  const status = license.status;
  const statusCls = getStatusColor(status);
  const licenseType = license.licenseType ?? "simple";
  const typeBadge = getLicenseTypeBadge(licenseType);
  const features = license.features ?? [];
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
            <p className={valueClass}>{license.customer}</p>
          </div>
          <div>
            <p className={labelClass}>Product</p>
            <p className={valueClass}>{license.product}</p>
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
            <p className={valueClass}>{license.createdAt}</p>
          </div>
          <div>
            <p className={labelClass}>Expires</p>
            <p className={valueClass}>{license.expiresAt}</p>
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
                            const result = await deactivateDevice(key, a.deviceId);
                            if (result.success) {
                              mutateActivations();
                              onMutate();
                              toast.success("Device deactivated");
                            }
                            setDeactivating(null);
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
          <Button variant="outline" size="sm" onClick={() => onExtend(key, license.expiresAt)}>
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

export function LicensesSection() {
  const { data: allLicenses, isLoading, mutate } = useLicenses();
  const [searchQuery, setSearchQuery] = useState("");
  const [statusFilter, setStatusFilter] = useState<string>("all");
  const [page, setPage] = useState(1);

  // Dialog state
  const [detailOpen, setDetailOpen] = useState(false);
  const [selectedLicense, setSelectedLicense] = useState<License | null>(null);
  const [signDialogLicense, setSignDialogLicense] = useState<License | null>(null);
  const [deleteTarget, setDeleteTarget] = useState<License | null>(null);
  const [deleteLoading, setDeleteLoading] = useState(false);
  const [copiedKey, setCopiedKey] = useState<string | null>(null);

  const licenses = (allLicenses ?? []) as unknown as License[];

  // Reset page when filters change
  useEffect(() => { setPage(1); }, [searchQuery, statusFilter]);

  const maskKey = (key: string) => {
    const lastSegment = key.slice(-4);
    return `\u2022\u2022\u2022\u2022-\u2022\u2022\u2022\u2022-\u2022\u2022\u2022\u2022-${lastSegment}`;
  };

  const counts = {
    total: licenses.length,
    active: licenses.filter((l) => l.status === "active").length,
    expired: licenses.filter((l) => l.status === "expired").length,
    revoked: licenses.filter((l) => l.status === "revoked").length,
  };

  const filteredLicenses = licenses.filter((l) => {
    const matchesSearch =
      l.key.toLowerCase().includes(searchQuery.toLowerCase()) ||
      l.customer.toLowerCase().includes(searchQuery.toLowerCase()) ||
      l.product.toLowerCase().includes(searchQuery.toLowerCase());
    const matchesStatus = statusFilter === "all" || l.status === statusFilter;
    return matchesSearch && matchesStatus;
  });

  // Paginate
  const totalPages = Math.max(1, Math.ceil(filteredLicenses.length / PER_PAGE));
  const paginatedLicenses = filteredLicenses.slice((page - 1) * PER_PAGE, page * PER_PAGE);

  // Action handlers
  const openDetail = (license: License) => {
    setSelectedLicense(license);
    setDetailOpen(true);
  };

  const closeDialog = () => {
    setDetailOpen(false);
    setSelectedLicense(null);
  };

  const copyKey = (key: string) => {
    navigator.clipboard.writeText(key);
    setCopiedKey(key);
    setTimeout(() => setCopiedKey(null), 2000);
  };

  const handleRevoke = async (key: string) => {
    const result = await updateLicense(key, { status: "revoked" });
    if (result.success) {
      mutate();
      toast.success("License revoked");
      closeDialog();
    }
  };

  const handleExtend = async (key: string, currentExpiry: string) => {
    const d = new Date(currentExpiry);
    d.setFullYear(d.getFullYear() + 1);
    const result = await updateLicense(key, { expiresAt: d.toISOString().split("T")[0] });
    if (result.success) {
      mutate();
      toast.success("License extended by 1 year");
      closeDialog();
    }
  };

  const handleDelete = async () => {
    if (!deleteTarget) return;
    setDeleteLoading(true);
    const result = await deleteLicense(deleteTarget.key);
    if (result.success) {
      mutate();
      toast.success("License deleted");
      setDeleteTarget(null);
    }
    setDeleteLoading(false);
  };

  if (isLoading) {
    return (
      <div className="space-y-6">
        <Skeleton className="h-12 w-full rounded-lg" />
        <div className="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-4 gap-4">
          {[...Array(4)].map((_, i) => <Skeleton key={i} className="h-24 rounded-xl" />)}
        </div>
        <Skeleton className="h-10 w-full" />
        <Skeleton className="h-[400px] rounded-xl" />
      </div>
    );
  }

  return (
    <div className="space-y-6">
      {/* Info banner */}
      <div className="flex items-center gap-3 px-4 py-3 rounded-lg bg-secondary border border-border text-sm text-muted-foreground">
        <KeyRound className="w-4 h-4 shrink-0 text-chart-1" />
        <span>Showing license keys for <span className="font-medium text-foreground">licensed products</span> only. Platform and API products are managed via their respective dashboards.</span>
      </div>

      {/* Summary cards */}
      <div className="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-4 gap-4">
        <MetricCard title="Total Licenses" value={String(counts.total)} change={`${counts.active} active`} changeType="neutral" icon={KeyRound} delay={0} />
        <MetricCard title="Active Licenses" value={String(counts.active)} change={`${Math.round((counts.active / (counts.total || 1)) * 100)}% of total`} changeType={counts.active > 0 ? "positive" : "neutral"} icon={CheckCircle2} delay={1} />
        <MetricCard title="Expired Licenses" value={String(counts.expired)} change={counts.expired > 0 ? `${counts.expired} pending renewal` : "None pending"} changeType="neutral" icon={Clock} delay={2} />
        <MetricCard title="Revoked Licenses" value={String(counts.revoked)} change={counts.revoked > 0 ? `${counts.revoked} revoked` : "None revoked"} changeType={counts.revoked > 0 ? "negative" : "neutral"} icon={XCircle} delay={3} />
      </div>

      {/* Filters and search */}
      <div className="flex flex-col sm:flex-row items-start sm:items-center justify-between gap-4">
        <div className="flex items-center gap-3">
          <div className="relative">
            <Search className="absolute left-3 top-1/2 -translate-y-1/2 w-4 h-4 text-muted-foreground" />
            <input
              type="text"
              placeholder="Search keys, customers, products..."
              value={searchQuery}
              onChange={(e) => setSearchQuery(e.target.value)}
              className="w-72 h-9 pl-9 pr-4 rounded-lg bg-secondary border border-border text-sm text-foreground placeholder:text-muted-foreground focus:outline-none focus:ring-2 focus:ring-ring/20 focus:border-accent transition-all duration-200"
            />
          </div>
          <div className="flex items-center gap-2">
            {["all", "active", "expired", "revoked", "suspended"].map((filter) => (
              <button
                key={filter}
                onClick={() => setStatusFilter(filter)}
                className={cn(
                  "px-3 py-1.5 rounded-lg text-xs font-medium transition-all duration-200",
                  statusFilter === filter
                    ? "bg-accent text-accent-foreground"
                    : "bg-secondary text-muted-foreground hover:text-foreground"
                )}
              >
                {filter.charAt(0).toUpperCase() + filter.slice(1)}
              </button>
            ))}
          </div>
        </div>
      </div>

      {/* License table */}
      <div className="bg-card border border-border rounded-xl overflow-hidden animate-in fade-in slide-in-from-bottom-4 duration-500">
        <div className="overflow-x-auto">
          <table className="w-full">
            <thead>
              <tr className="border-b border-border bg-secondary/50">
                <th className="text-left py-3 px-4 text-xs font-semibold text-muted-foreground uppercase tracking-wider">License Key</th>
                <th className="text-left py-3 px-4 text-xs font-semibold text-muted-foreground uppercase tracking-wider">Customer</th>
                <th className="text-left py-3 px-4 text-xs font-semibold text-muted-foreground uppercase tracking-wider">Product</th>
                <th className="text-left py-3 px-4 text-xs font-semibold text-muted-foreground uppercase tracking-wider">Status</th>
                <th className="text-left py-3 px-4 text-xs font-semibold text-muted-foreground uppercase tracking-wider">Type</th>
                <th className="text-left py-3 px-4 text-xs font-semibold text-muted-foreground uppercase tracking-wider">Created</th>
                <th className="text-left py-3 px-4 text-xs font-semibold text-muted-foreground uppercase tracking-wider">Expires</th>
                <th className="w-12" />
              </tr>
            </thead>
            <tbody>
              {paginatedLicenses.map((license, index) => {
                const StatusIcon = statusIcons[license.status];
                const statusColor = getStatusColor(license.status);
                const licenseType = license.licenseType ?? "simple";
                const typeBadge = getLicenseTypeBadge(licenseType);
                const hasCertificate = !!license.hasCertificate;

                return (
                  <tr
                    key={license.key}
                    className="border-b border-border last:border-0 hover:bg-secondary/30 transition-colors duration-150 cursor-pointer animate-in fade-in slide-in-from-left-2"
                    style={{ animationDelay: `${index * 50}ms`, animationFillMode: "both" }}
                    onClick={() => openDetail(license)}
                  >
                    <td className="py-4 px-4" onClick={(e) => e.stopPropagation()}>
                      <button onClick={() => copyKey(license.key)} className="inline-flex items-center gap-1.5 px-2 py-1 rounded-md bg-secondary hover:bg-secondary/80 text-xs font-mono text-muted-foreground hover:text-foreground transition-colors">
                        {hasCertificate ? (
                          <Shield className="w-3 h-3 shrink-0 text-chart-1" />
                        ) : (
                          <KeyRound className="w-3 h-3 shrink-0" />
                        )}
                        <span>{maskKey(license.key)}</span>
                        {copiedKey === license.key ? <Check className="w-3 h-3 text-success shrink-0" /> : <Copy className="w-3 h-3 shrink-0" />}
                      </button>
                    </td>
                    <td className="py-4 px-4">
                      <span className="text-sm font-medium text-foreground">{license.customer}</span>
                    </td>
                    <td className="py-4 px-4">
                      <span className="px-2 py-1 rounded-md bg-secondary text-xs font-medium text-foreground">{license.product}</span>
                    </td>
                    <td className="py-4 px-4">
                      <div className={cn("inline-flex items-center gap-1 px-2 py-1 rounded-md text-xs font-medium", statusColor)}>
                        <StatusIcon className="w-3 h-3" />
                        {license.status.charAt(0).toUpperCase() + license.status.slice(1)}
                      </div>
                    </td>
                    <td className="py-4 px-4">
                      <span className={cn("inline-flex items-center gap-1 px-2 py-0.5 rounded-md text-xs font-medium", typeBadge.className)}>
                        {licenseType === "signed" && <Shield className="w-3 h-3" />}
                        {typeBadge.label}
                      </span>
                    </td>
                    <td className="py-4 px-4">
                      <span className="text-sm text-muted-foreground">{license.createdAt}</span>
                    </td>
                    <td className="py-4 px-4">
                      <span className="text-sm text-muted-foreground">{license.expiresAt}</span>
                    </td>
                    <td className="py-4 px-4" onClick={(e) => e.stopPropagation()}>
                      <DropdownMenu>
                        <DropdownMenuTrigger asChild>
                          <button className="w-8 h-8 flex items-center justify-center rounded-lg text-muted-foreground hover:text-foreground hover:bg-secondary transition-colors">
                            <MoreHorizontal className="w-4 h-4" />
                          </button>
                        </DropdownMenuTrigger>
                        <DropdownMenuContent align="end">
                          <DropdownMenuItem onClick={() => copyKey(license.key)}>
                            <Copy className="w-4 h-4" /> Copy Key
                          </DropdownMenuItem>
                          {!hasCertificate ? (
                            <DropdownMenuItem onClick={() => setSignDialogLicense(license)}>
                              <FileSignature className="w-4 h-4" /> Generate Certificate
                            </DropdownMenuItem>
                          ) : (
                            <>
                              <DropdownMenuItem asChild>
                                <a href={getLicenseExportUrl(license.key)} download onClick={(e) => e.stopPropagation()}>
                                  <Download className="w-4 h-4" /> Download .lic
                                </a>
                              </DropdownMenuItem>
                              <DropdownMenuItem onClick={() => setSignDialogLicense(license)}>
                                <FileSignature className="w-4 h-4" /> Re-sign
                              </DropdownMenuItem>
                            </>
                          )}
                          <DropdownMenuSeparator />
                          {license.status === "active" && (
                            <DropdownMenuItem onClick={() => handleRevoke(license.key)}>
                              <ShieldOff className="w-4 h-4" /> Revoke
                            </DropdownMenuItem>
                          )}
                          <DropdownMenuItem onClick={() => handleExtend(license.key, license.expiresAt)}>
                            <CalendarPlus className="w-4 h-4" /> Extend 1 Year
                          </DropdownMenuItem>
                          <DropdownMenuSeparator />
                          <DropdownMenuItem variant="destructive" onClick={() => setDeleteTarget(license)}>
                            <Trash2 className="w-4 h-4" /> Delete
                          </DropdownMenuItem>
                        </DropdownMenuContent>
                      </DropdownMenu>
                    </td>
                  </tr>
                );
              })}
            </tbody>
          </table>
        </div>

        {/* Pagination */}
        <div className="flex items-center justify-between px-4 py-3 border-t border-border bg-secondary/30">
          <span className="text-sm text-muted-foreground">
            Showing {filteredLicenses.length > 0 ? (page - 1) * PER_PAGE + 1 : 0}–{Math.min(page * PER_PAGE, filteredLicenses.length)} of {filteredLicenses.length} licenses
          </span>
          <div className="flex items-center gap-1">
            <button
              onClick={() => setPage((p) => Math.max(1, p - 1))}
              disabled={page <= 1}
              className="px-2 py-1.5 rounded-lg text-sm text-muted-foreground hover:text-foreground hover:bg-secondary transition-colors duration-200 disabled:opacity-40 disabled:pointer-events-none"
            >
              <ChevronLeft className="w-4 h-4" />
            </button>
            {Array.from({ length: totalPages }, (_, i) => i + 1)
              .slice(Math.max(0, page - 3), page + 2)
              .map((p) => (
                <button
                  key={p}
                  onClick={() => setPage(p)}
                  className={cn(
                    "px-3 py-1.5 rounded-lg text-sm font-medium transition-colors duration-200",
                    p === page
                      ? "bg-accent text-accent-foreground"
                      : "text-muted-foreground hover:text-foreground hover:bg-secondary"
                  )}
                >
                  {p}
                </button>
              ))}
            <button
              onClick={() => setPage((p) => Math.min(totalPages, p + 1))}
              disabled={page >= totalPages}
              className="px-2 py-1.5 rounded-lg text-sm text-muted-foreground hover:text-foreground hover:bg-secondary transition-colors duration-200 disabled:opacity-40 disabled:pointer-events-none"
            >
              <ChevronRight className="w-4 h-4" />
            </button>
          </div>
        </div>
      </div>

      {/* Detail Dialog */}
      <Dialog open={detailOpen} onOpenChange={(open) => { if (!open) closeDialog(); }}>
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
      <DeleteDialog
        open={!!deleteTarget}
        onOpenChange={(open) => { if (!open) setDeleteTarget(null); }}
        title="Delete license?"
        description={`This will permanently delete license key "${deleteTarget?.key ?? ""}".`}
        onConfirm={handleDelete}
        loading={deleteLoading}
      />
    </div>
  );
}

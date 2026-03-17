"use client";

import { useState } from "react";
import { cn } from "@/lib/utils";
import { Plus, Search, MoreHorizontal, Pencil, Trash2, Eye, EyeOff, Globe, Copy } from "lucide-react";
import { Table, TableBody, TableCell, TableHead, TableHeader, TableRow } from "@/components/ui/table";
import { Dialog, DialogContent, DialogHeader, DialogTitle, DialogDescription, DialogFooter } from "@/components/ui/dialog";
import { DropdownMenu, DropdownMenuContent, DropdownMenuItem, DropdownMenuTrigger, DropdownMenuSeparator } from "@/components/ui/dropdown-menu";
import { Button } from "@/components/ui/button";
import { useWebhooks, createWebhook, updateWebhook, deleteWebhook } from "@/hooks/use-api";
import { toast } from "sonner";
import { Skeleton } from "@/components/ui/skeleton";
import { DeleteDialog } from "./delete-dialog";

type Webhook = Record<string, unknown>;
type DialogMode = "view" | "edit" | "create";

const EVENT_GROUPS = [
  {
    label: "Invoice",
    events: ["invoice.created", "invoice.issued", "invoice.paid", "invoice.overdue", "invoice.voided"],
  },
  {
    label: "Payment",
    events: ["payment.received", "payment.refunded"],
  },
  {
    label: "Subscription",
    events: ["subscription.created", "subscription.renewed", "subscription.canceled", "subscription.paused"],
  },
  {
    label: "Dunning",
    events: ["dunning.reminder", "dunning.warning", "dunning.final_notice", "dunning.suspension"],
  },
];

function WebhookDetail({ webhook, onEdit, onDelete }: { webhook: Webhook; onEdit: () => void; onDelete: () => void }) {
  const [showSecret, setShowSecret] = useState(false);
  const labelClass = "text-xs text-muted-foreground uppercase tracking-wider";
  const events = (webhook.events as string[]) ?? [];

  return (
    <div>
      <DialogHeader>
        <div className="flex items-center gap-3">
          <Globe className="w-5 h-5 text-accent" />
          <div>
            <DialogTitle className="text-lg">{(webhook.url as string)}</DialogTitle>
            <DialogDescription>{(webhook.description as string) || "No description"}</DialogDescription>
          </div>
        </div>
      </DialogHeader>

      <div className="mt-6 space-y-5">
        <div className="grid grid-cols-2 gap-4">
          <div>
            <p className={labelClass}>Status</p>
            <span className={cn("inline-flex px-2 py-0.5 rounded-full text-xs font-medium capitalize mt-1",
              webhook.status === "active" ? "bg-sky-500/20 text-sky-400" : "bg-muted-foreground/20 text-muted-foreground"
            )}>
              {webhook.status as string}
            </span>
          </div>
          <div>
            <p className={labelClass}>Created</p>
            <p className="text-sm font-medium text-foreground mt-0.5">
              {new Date(webhook.createdAt as string).toLocaleDateString()}
            </p>
          </div>
        </div>

        <div>
          <p className={labelClass}>Signing Secret</p>
          <div className="flex items-center gap-2 mt-1">
            <code className="flex-1 text-sm font-mono bg-secondary px-3 py-1.5 rounded-lg text-foreground">
              {showSecret ? (webhook.secret as string) : "••••••••••••••••••••••••"}
            </code>
            <Button variant="ghost" size="icon" className="h-8 w-8" onClick={() => setShowSecret(!showSecret)}>
              {showSecret ? <EyeOff className="w-4 h-4" /> : <Eye className="w-4 h-4" />}
            </Button>
            <Button variant="ghost" size="icon" className="h-8 w-8" onClick={() => {
              navigator.clipboard.writeText(webhook.secret as string);
              toast.success("Secret copied");
            }}>
              <Copy className="w-4 h-4" />
            </Button>
          </div>
        </div>

        <div>
          <p className={cn(labelClass, "mb-2")}>Subscribed Events ({events.includes("*") ? "All" : events.length})</p>
          <div className="flex flex-wrap gap-1.5">
            {events.map((e) => (
              <span key={e} className="inline-flex px-2 py-0.5 rounded-md text-[10px] font-medium bg-secondary text-muted-foreground">
                {e}
              </span>
            ))}
          </div>
        </div>
      </div>

      <div className="flex items-center justify-between pt-6 mt-6 border-t border-border">
        <Button variant="outline" size="sm" className="text-destructive hover:text-destructive hover:bg-destructive/10" onClick={onDelete}>
          <Trash2 className="w-4 h-4 mr-1.5" /> Delete
        </Button>
        <Button size="sm" onClick={onEdit}>
          <Pencil className="w-4 h-4 mr-1.5" /> Edit
        </Button>
      </div>
    </div>
  );
}

function WebhookForm({ webhook, onClose, onSuccess }: { webhook: Webhook | null; onClose: () => void; onSuccess: () => void }) {
  const isEditing = !!webhook;
  const [url, setUrl] = useState((webhook?.url as string) ?? "");
  const [description, setDescription] = useState((webhook?.description as string) ?? "");
  const [status, setStatus] = useState((webhook?.status as string) ?? "active");
  const [selectedEvents, setSelectedEvents] = useState<string[]>(
    (webhook?.events as string[]) ?? []
  );
  const [allEvents, setAllEvents] = useState(selectedEvents.includes("*"));
  const [submitting, setSubmitting] = useState(false);

  const inputClass = "w-full h-9 mt-1 px-3 rounded-lg bg-secondary border border-border text-sm text-foreground focus:outline-none focus:ring-2 focus:ring-ring/20 focus:border-accent";
  const labelClass = "text-xs font-medium text-muted-foreground uppercase tracking-wider";

  const toggleEvent = (event: string) => {
    setSelectedEvents((prev) =>
      prev.includes(event) ? prev.filter((e) => e !== event) : [...prev, event]
    );
  };

  const handleSubmit = async () => {
    if (!url.trim()) {
      toast.error("URL is required");
      return;
    }
    const events = allEvents ? ["*"] : selectedEvents;
    if (events.length === 0) {
      toast.error("Select at least one event");
      return;
    }
    setSubmitting(true);
    const data = { url, description: description || null, events, status };
    if (isEditing) {
      const result = await updateWebhook(webhook.id as string, data);
      if (result.success) {
        toast.success("Webhook updated");
        onSuccess();
        onClose();
      }
    } else {
      const result = await createWebhook(data);
      if (result.success) {
        toast.success("Webhook created");
        onSuccess();
        onClose();
      }
    }
    setSubmitting(false);
  };

  return (
    <div className="space-y-4 max-h-[70vh] overflow-y-auto pr-1">
      <div>
        <label className={labelClass}>Endpoint URL</label>
        <input value={url} onChange={(e) => setUrl(e.target.value)} className={inputClass} placeholder="https://example.com/webhook" />
      </div>
      <div>
        <label className={labelClass}>Description</label>
        <input value={description} onChange={(e) => setDescription(e.target.value)} className={inputClass} placeholder="Optional description" />
      </div>
      <div>
        <label className={labelClass}>Status</label>
        <div className="flex gap-2 mt-1">
          {(["active", "inactive"] as const).map((s) => (
            <button key={s} onClick={() => setStatus(s)} className={cn("px-3 py-1.5 rounded-lg text-xs font-medium transition-all capitalize", status === s ? "bg-accent text-accent-foreground" : "bg-secondary text-muted-foreground hover:text-foreground")}>
              {s}
            </button>
          ))}
        </div>
      </div>
      <div>
        <label className={labelClass}>Events</label>
        <div className="mt-2 space-y-3">
          <label className="flex items-center gap-2 px-3 py-2 bg-accent/10 rounded-lg cursor-pointer">
            <input type="checkbox" checked={allEvents} onChange={(e) => { setAllEvents(e.target.checked); if (e.target.checked) setSelectedEvents([]); }} className="h-4 w-4 rounded border-border" />
            <span className="text-sm font-medium text-foreground">All events (*)</span>
          </label>
          {!allEvents && EVENT_GROUPS.map((group) => (
            <div key={group.label}>
              <p className="text-[10px] text-muted-foreground uppercase mb-1.5">{group.label}</p>
              <div className="grid grid-cols-2 gap-1.5">
                {group.events.map((event) => (
                  <label key={event} className="flex items-center gap-2 px-2.5 py-1.5 bg-secondary/50 rounded-lg cursor-pointer hover:bg-secondary">
                    <input type="checkbox" checked={selectedEvents.includes(event)} onChange={() => toggleEvent(event)} className="h-3.5 w-3.5 rounded border-border" />
                    <span className="text-xs text-foreground">{event}</span>
                  </label>
                ))}
              </div>
            </div>
          ))}
        </div>
      </div>
      <DialogFooter className="pt-4 border-t border-border">
        <Button variant="outline" onClick={onClose}>Cancel</Button>
        <Button onClick={handleSubmit} disabled={submitting}>{submitting ? "Saving..." : isEditing ? "Save Changes" : "Create Webhook"}</Button>
      </DialogFooter>
    </div>
  );
}

export function ManageWebhooksSection() {
  const { data: webhookList, isLoading, mutate } = useWebhooks();
  const [search, setSearch] = useState("");
  const [dialogOpen, setDialogOpen] = useState(false);
  const [dialogMode, setDialogMode] = useState<DialogMode>("view");
  const [selected, setSelected] = useState<Webhook | null>(null);
  const [deleteTarget, setDeleteTarget] = useState<Webhook | null>(null);
  const [deleteLoading, setDeleteLoading] = useState(false);

  const webhooks = (webhookList ?? []) as Webhook[];
  const filtered = webhooks.filter((w) =>
    ((w.url as string) ?? "").toLowerCase().includes(search.toLowerCase()) ||
    ((w.description as string) ?? "").toLowerCase().includes(search.toLowerCase())
  );

  const openDetail = (webhook: Webhook) => { setSelected(webhook); setDialogMode("view"); setDialogOpen(true); };
  const openEdit = (webhook: Webhook) => { setSelected(webhook); setDialogMode("edit"); setDialogOpen(true); };
  const openCreate = () => { setSelected(null); setDialogMode("create"); setDialogOpen(true); };
  const closeDialog = () => { setDialogOpen(false); setSelected(null); setDialogMode("view"); };

  const handleDelete = async () => {
    if (!deleteTarget) return;
    setDeleteLoading(true);
    const result = await deleteWebhook(deleteTarget.id as string);
    if (result.success) {
      mutate();
      toast.success("Webhook deleted");
      setDeleteTarget(null);
    }
    setDeleteLoading(false);
  };

  if (isLoading) {
    return (
      <div className="space-y-6">
        <Skeleton className="h-10 w-full" />
        <Skeleton className="h-[300px] rounded-xl" />
      </div>
    );
  }

  return (
    <div className="space-y-6">
      <div className="flex flex-col sm:flex-row items-start sm:items-center justify-between gap-4">
        <div className="relative">
          <Search className="absolute left-3 top-1/2 -translate-y-1/2 w-4 h-4 text-muted-foreground" />
          <input type="text" placeholder="Search webhooks..." value={search} onChange={(e) => setSearch(e.target.value)} className="w-72 h-9 pl-9 pr-4 rounded-lg bg-secondary border border-border text-sm text-foreground placeholder:text-muted-foreground focus:outline-none focus:ring-2 focus:ring-ring/20 focus:border-accent transition-all" />
        </div>
        <Button onClick={openCreate}>
          <Plus className="w-4 h-4 mr-2" /> Add Webhook
        </Button>
      </div>

      <div className="bg-card border border-border rounded-xl overflow-hidden">
        <Table>
          <TableHeader>
            <TableRow className="bg-secondary/50">
              <TableHead className="px-4 text-xs font-semibold uppercase tracking-wider">URL</TableHead>
              <TableHead className="px-4 text-xs font-semibold uppercase tracking-wider">Description</TableHead>
              <TableHead className="px-4 text-xs font-semibold uppercase tracking-wider">Events</TableHead>
              <TableHead className="px-4 text-xs font-semibold uppercase tracking-wider">Status</TableHead>
              <TableHead className="px-4 text-xs font-semibold uppercase tracking-wider">Created</TableHead>
              <TableHead className="w-12" />
            </TableRow>
          </TableHeader>
          <TableBody>
            {filtered.length === 0 ? (
              <TableRow>
                <TableCell colSpan={6} className="text-center py-8 text-muted-foreground">No webhooks configured</TableCell>
              </TableRow>
            ) : filtered.map((webhook) => {
              const events = (webhook.events as string[]) ?? [];
              return (
                <TableRow key={webhook.id as string} className="cursor-pointer" onClick={() => openDetail(webhook)}>
                  <TableCell className="px-4 font-mono text-xs max-w-[250px] truncate">{webhook.url as string}</TableCell>
                  <TableCell className="px-4 text-sm text-muted-foreground">{(webhook.description as string) || "—"}</TableCell>
                  <TableCell className="px-4">
                    <span className="inline-flex px-2 py-0.5 rounded-md text-xs font-medium bg-secondary text-muted-foreground">
                      {events.includes("*") ? "All" : `${events.length} events`}
                    </span>
                  </TableCell>
                  <TableCell className="px-4">
                    <span className={cn("px-2 py-0.5 rounded-full text-xs font-medium capitalize",
                      webhook.status === "active" ? "bg-sky-500/20 text-sky-400" : "bg-muted-foreground/20 text-muted-foreground"
                    )}>
                      {webhook.status as string}
                    </span>
                  </TableCell>
                  <TableCell className="px-4 text-xs text-muted-foreground">
                    {new Date(webhook.createdAt as string).toLocaleDateString()}
                  </TableCell>
                  <TableCell className="px-4" onClick={(e) => e.stopPropagation()}>
                    <DropdownMenu>
                      <DropdownMenuTrigger asChild>
                        <button className="w-8 h-8 flex items-center justify-center rounded-lg text-muted-foreground hover:text-foreground hover:bg-secondary transition-colors">
                          <MoreHorizontal className="w-4 h-4" />
                        </button>
                      </DropdownMenuTrigger>
                      <DropdownMenuContent align="end">
                        <DropdownMenuItem onClick={() => openEdit(webhook)}>
                          <Pencil className="w-4 h-4" /> Edit
                        </DropdownMenuItem>
                        <DropdownMenuSeparator />
                        <DropdownMenuItem variant="destructive" onClick={() => setDeleteTarget(webhook)}>
                          <Trash2 className="w-4 h-4" /> Delete
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
          <span className="text-sm text-muted-foreground">Showing {filtered.length} of {webhooks.length} webhooks</span>
        </div>
      </div>

      <Dialog open={dialogOpen} onOpenChange={(open) => { if (!open) closeDialog(); }}>
        <DialogContent className="sm:max-w-2xl max-h-[90vh] overflow-y-auto">
          {dialogMode === "view" && selected ? (
            <WebhookDetail
              webhook={selected}
              onEdit={() => setDialogMode("edit")}
              onDelete={() => { closeDialog(); setDeleteTarget(selected); }}
            />
          ) : (
            <>
              <DialogHeader>
                <DialogTitle>{selected ? "Edit Webhook" : "Add Webhook"}</DialogTitle>
                <DialogDescription>{selected ? "Update webhook configuration" : "Register a new webhook endpoint"}</DialogDescription>
              </DialogHeader>
              <WebhookForm key={selected?.id as string ?? "new"} webhook={selected} onClose={closeDialog} onSuccess={() => mutate()} />
            </>
          )}
        </DialogContent>
      </Dialog>

      <DeleteDialog
        open={!!deleteTarget}
        onOpenChange={(open) => { if (!open) setDeleteTarget(null); }}
        title="Delete Webhook?"
        description={`This will permanently delete the webhook endpoint and all delivery history.`}
        onConfirm={handleDelete}
        loading={deleteLoading}
      />
    </div>
  );
}

"use client";

import { useBillingEvents } from "@/hooks/use-api";
import {
  Popover,
  PopoverContent,
  PopoverTrigger,
} from "@/components/ui/popover";
import {
  Bell,
  FileText,
  DollarSign,
  RefreshCw,
  AlertTriangle,
  CheckCircle2,
} from "lucide-react";

const eventLabels: Record<string, string> = {
  "invoice.created": "Invoice created",
  "invoice.issued": "Invoice issued",
  "invoice.paid": "Invoice paid",
  "invoice.overdue": "Invoice overdue",
  "invoice.voided": "Invoice voided",
  "payment.received": "Payment received",
  "payment.refunded": "Payment refunded",
  "subscription.created": "Subscription created",
  "subscription.renewed": "Subscription renewed",
  "subscription.canceled": "Subscription canceled",
  "subscription.paused": "Subscription paused",
  "dunning.reminder": "Payment reminder sent",
  "dunning.warning": "Payment warning sent",
  "dunning.final_notice": "Final payment notice",
  "dunning.suspension": "Account suspended",
};

const eventIcons: Record<string, React.ElementType> = {
  "invoice.created": FileText,
  "invoice.issued": FileText,
  "invoice.paid": CheckCircle2,
  "invoice.overdue": AlertTriangle,
  "invoice.voided": FileText,
  "payment.received": DollarSign,
  "payment.refunded": DollarSign,
  "subscription.created": RefreshCw,
  "subscription.renewed": RefreshCw,
  "subscription.canceled": RefreshCw,
  "subscription.paused": RefreshCw,
  "dunning.reminder": AlertTriangle,
  "dunning.warning": AlertTriangle,
  "dunning.final_notice": AlertTriangle,
  "dunning.suspension": AlertTriangle,
};

function formatRelativeTime(dateStr: string): string {
  const date = new Date(dateStr);
  const now = new Date();
  const diffMs = now.getTime() - date.getTime();
  const diffMins = Math.floor(diffMs / 60000);
  if (diffMins < 1) return "just now";
  if (diffMins < 60) return `${diffMins}m ago`;
  const diffHours = Math.floor(diffMins / 60);
  if (diffHours < 24) return `${diffHours}h ago`;
  const diffDays = Math.floor(diffHours / 24);
  if (diffDays < 7) return `${diffDays}d ago`;
  return date.toLocaleDateString("en-US", { month: "short", day: "numeric" });
}

export function NotificationCenter() {
  const { data: events } = useBillingEvents(undefined, 10);
  const raw = events?.data ?? events;
  const eventList = (Array.isArray(raw) ? raw : []) as {
    id: string;
    eventType: string;
    resourceType: string;
    resourceId: string;
    customerName?: string;
    data?: Record<string, unknown>;
    createdAt: string;
  }[];

  const hasEvents = eventList.length > 0;

  return (
    <Popover>
      <PopoverTrigger asChild>
        <button className="relative w-9 h-9 flex items-center justify-center rounded-lg text-muted-foreground hover:text-foreground hover:bg-secondary transition-all duration-200">
          <Bell className="w-5 h-5" />
          {hasEvents && (
            <span className="absolute top-1.5 right-1.5 w-2 h-2 bg-accent rounded-full animate-pulse" />
          )}
        </button>
      </PopoverTrigger>
      <PopoverContent className="w-80 p-0" align="end">
        <div className="px-4 py-3 border-b border-border">
          <h3 className="text-sm font-semibold text-foreground">Notifications</h3>
          <p className="text-xs text-muted-foreground">Recent billing activity</p>
        </div>
        <div className="max-h-[360px] overflow-y-auto">
          {eventList.length === 0 ? (
            <div className="px-4 py-8 text-center text-sm text-muted-foreground">
              No recent activity
            </div>
          ) : (
            <div className="divide-y divide-border">
              {eventList.map((event) => {
                const Icon = eventIcons[event.eventType] ?? Bell;
                const label = eventLabels[event.eventType] ?? event.eventType;
                const isWarning = event.eventType.startsWith("dunning.") || event.eventType === "invoice.overdue";

                return (
                  <div key={event.id} className="px-4 py-3 hover:bg-secondary/50 transition-colors">
                    <div className="flex items-start gap-3">
                      <div className={`w-8 h-8 rounded-lg flex items-center justify-center shrink-0 ${
                        isWarning ? "bg-destructive/10" : "bg-accent/10"
                      }`}>
                        <Icon className={`w-4 h-4 ${isWarning ? "text-destructive" : "text-accent"}`} />
                      </div>
                      <div className="min-w-0 flex-1">
                        <p className="text-sm font-medium text-foreground">{label}</p>
                        <p className="text-xs text-muted-foreground truncate">
                          {event.customerName ?? event.resourceType} &middot; {event.resourceId.slice(0, 8)}
                        </p>
                      </div>
                      <span className="text-xs text-muted-foreground shrink-0">
                        {formatRelativeTime(event.createdAt)}
                      </span>
                    </div>
                  </div>
                );
              })}
            </div>
          )}
        </div>
      </PopoverContent>
    </Popover>
  );
}

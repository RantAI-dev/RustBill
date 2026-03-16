"use client";

import { Users } from "lucide-react";

export function TeamSection() {
  return (
    <div className="space-y-6">
      {/* Empty state */}
      <div className="flex flex-col items-center justify-center py-20 bg-card border border-border rounded-xl">
        <div className="w-16 h-16 rounded-2xl bg-secondary flex items-center justify-center mb-4">
          <Users className="w-8 h-8 text-muted-foreground" />
        </div>
        <h3 className="text-lg font-semibold text-foreground mb-2">No team members yet</h3>
        <p className="text-sm text-muted-foreground text-center max-w-md">
          Team performance tracking will appear here once team member data is available.
        </p>
      </div>
    </div>
  );
}

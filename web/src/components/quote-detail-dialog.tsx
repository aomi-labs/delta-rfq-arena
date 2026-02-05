"use client";

import { useEffect, useState } from "react";
import {
  Dialog,
  DialogContent,
  DialogHeader,
  DialogTitle,
  DialogDescription,
} from "@/components/ui/dialog";
import { Badge } from "@/components/ui/badge";
import type { Quote, FillReceipt } from "@/types/api";
import { formatDistanceToNow, formatUnits } from "@/lib/format";

interface QuoteDetailDialogProps {
  quote: Quote | null;
  open: boolean;
  onOpenChange: (open: boolean) => void;
  onFetchReceipts: (quoteId: string) => Promise<FillReceipt[]>;
}

export function QuoteDetailDialog({
  quote,
  open,
  onOpenChange,
  onFetchReceipts,
}: QuoteDetailDialogProps) {
  const [receipts, setReceipts] = useState<FillReceipt[]>([]);
  const [loadingReceipts, setLoadingReceipts] = useState(false);

  useEffect(() => {
    if (quote && open) {
      setLoadingReceipts(true);
      onFetchReceipts(quote.id)
        .then(setReceipts)
        .catch(console.error)
        .finally(() => setLoadingReceipts(false));
    }
  }, [quote, open, onFetchReceipts]);

  if (!quote) return null;

  const isExpired = new Date(quote.expires_at) < new Date();
  const status = isExpired ? "expired" : quote.status;
  const sideColor = quote.spec.side === "buy" ? "text-green-600" : "text-red-600";

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="max-w-3xl max-h-[90vh] overflow-y-auto">
        <DialogHeader>
          <DialogTitle className="flex items-center gap-3">
            <span className={sideColor}>{quote.spec.side.toUpperCase()}</span>
            {quote.spec.size} {quote.spec.asset}
            <Badge
              variant={
                status === "active"
                  ? "success"
                  : status === "filled"
                    ? "default"
                    : "warning"
              }
            >
              {status}
            </Badge>
          </DialogTitle>
          <DialogDescription className="text-left">
            Quote ID: <span className="font-mono">{quote.id}</span>
          </DialogDescription>
        </DialogHeader>

        <div className="space-y-6">
          {/* Original Text */}
          <div className="p-4 bg-muted rounded-lg">
            <h4 className="text-sm font-medium mb-2">Original Quote (ESC)</h4>
            <p className="text-sm italic">&ldquo;{quote.original_text}&rdquo;</p>
          </div>

          {/* Spec & Constraints */}
          <div className="grid grid-cols-2 gap-6">
            <div>
              <h4 className="text-sm font-medium mb-3">Quote Spec</h4>
              <dl className="space-y-2 text-sm">
                <div className="flex justify-between">
                  <dt className="text-muted-foreground">Asset</dt>
                  <dd className="font-medium">{quote.spec.asset}</dd>
                </div>
                <div className="flex justify-between">
                  <dt className="text-muted-foreground">Size</dt>
                  <dd className="font-medium">{quote.spec.size}</dd>
                </div>
                <div className="flex justify-between">
                  <dt className="text-muted-foreground">Side</dt>
                  <dd className={sideColor}>{quote.spec.side}</dd>
                </div>
                <div className="flex justify-between">
                  <dt className="text-muted-foreground">Limit Price</dt>
                  <dd className="font-medium">
                    {quote.spec.limit_price ?? "Market"} {quote.spec.currency}
                  </dd>
                </div>
                <div className="flex justify-between">
                  <dt className="text-muted-foreground">Maker</dt>
                  <dd className="font-mono text-xs">{quote.maker_owner_id}</dd>
                </div>
              </dl>
            </div>

            <div>
              <h4 className="text-sm font-medium mb-3">Guardrails</h4>
              <dl className="space-y-2 text-sm">
                <div className="flex justify-between">
                  <dt className="text-muted-foreground">Max Debit</dt>
                  <dd className="font-mono text-xs">{formatUnits(quote.constraints.max_debit)}</dd>
                </div>
                <div className="flex justify-between">
                  <dt className="text-muted-foreground">Expires</dt>
                  <dd>{formatDistanceToNow(new Date(quote.expires_at))}</dd>
                </div>
                <div className="flex justify-between">
                  <dt className="text-muted-foreground">Allowed Feeds</dt>
                  <dd className="flex gap-1">
                    {quote.constraints.allowed_sources.map((s) => (
                      <Badge key={s} variant="outline" className="text-[10px]">
                        {s}
                      </Badge>
                    ))}
                  </dd>
                </div>
                <div className="flex justify-between">
                  <dt className="text-muted-foreground">Feed Freshness</dt>
                  <dd>&lt;{quote.constraints.max_staleness_secs}s</dd>
                </div>
                <div className="flex justify-between">
                  <dt className="text-muted-foreground">Quorum</dt>
                  <dd>
                    {quote.constraints.quorum_count} sources, {quote.constraints.quorum_tolerance_percent}% tolerance
                  </dd>
                </div>
                <div className="flex justify-between">
                  <dt className="text-muted-foreground">Atomic DvP</dt>
                  <dd>{quote.constraints.require_atomic_dvp ? "Required" : "No"}</dd>
                </div>
                <div className="flex justify-between">
                  <dt className="text-muted-foreground">Side Payments</dt>
                  <dd>{quote.constraints.no_side_payments ? "Blocked" : "Allowed"}</dd>
                </div>
              </dl>
            </div>
          </div>

          {/* Fill Attempts */}
          <div>
            <h4 className="text-sm font-medium mb-3">
              Fill Attempts ({receipts.length})
            </h4>
            {loadingReceipts ? (
              <p className="text-sm text-muted-foreground">Loading...</p>
            ) : receipts.length === 0 ? (
              <p className="text-sm text-muted-foreground">No fill attempts yet</p>
            ) : (
              <div className="space-y-2 max-h-60 overflow-y-auto">
                {receipts.map((receipt) => (
                  <div
                    key={receipt.receipt_id}
                    className={`p-3 rounded-lg border ${
                      receipt.result.status === "accepted"
                        ? "border-green-500/50 bg-green-500/10"
                        : "border-red-500/50 bg-red-500/10"
                    }`}
                  >
                    <div className="flex items-center justify-between mb-2">
                      <span className="font-mono text-xs">
                        {receipt.fill_attempt.taker_owner_id}
                      </span>
                      <Badge
                        variant={
                          receipt.result.status === "accepted" ? "success" : "destructive"
                        }
                      >
                        {receipt.result.status}
                      </Badge>
                    </div>
                    <div className="text-xs text-muted-foreground">
                      Size: {receipt.fill_attempt.size} @ {receipt.fill_attempt.price}
                    </div>
                    {receipt.result.status === "rejected" && (
                      <div className="mt-2 text-xs text-red-600">
                        Reason: {receipt.result.reason.code}
                      </div>
                    )}
                    {receipt.result.status === "accepted" && (
                      <div className="mt-2 text-xs text-green-600">
                        SDL: {receipt.result.sdl_hash.slice(0, 16)}...
                      </div>
                    )}
                  </div>
                ))}
              </div>
            )}
          </div>
        </div>
      </DialogContent>
    </Dialog>
  );
}

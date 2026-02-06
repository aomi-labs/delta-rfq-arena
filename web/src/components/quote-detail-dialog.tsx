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
import { formatUnixDistanceToNow, formatUnits, isExpired, formatTimestamp } from "@/lib/format";

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

  const expired = isExpired(quote.expires_at);
  const status = expired ? "expired" : quote.status;
  const directionColor = quote.direction === "buy" ? "text-green-600" : "text-red-600";

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="max-w-3xl max-h-[90vh] overflow-y-auto">
        <DialogHeader>
          <DialogTitle className="flex items-center gap-3">
            <span className={directionColor}>{quote.direction.toUpperCase()}</span>
            {quote.size} {quote.asset}
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
            <h4 className="text-sm font-medium mb-2">Original Quote</h4>
            <p className="text-sm italic">&ldquo;{quote.text}&rdquo;</p>
          </div>

          {/* Spec & Constraints */}
          <div className="grid grid-cols-2 gap-6">
            <div>
              <h4 className="text-sm font-medium mb-3">Quote Details</h4>
              <dl className="space-y-2 text-sm">
                <div className="flex justify-between">
                  <dt className="text-muted-foreground">Asset</dt>
                  <dd className="font-medium">{quote.asset}</dd>
                </div>
                <div className="flex justify-between">
                  <dt className="text-muted-foreground">Size</dt>
                  <dd className="font-medium">{quote.size}</dd>
                </div>
                <div className="flex justify-between">
                  <dt className="text-muted-foreground">Direction</dt>
                  <dd className={directionColor}>{quote.direction}</dd>
                </div>
                <div className="flex justify-between">
                  <dt className="text-muted-foreground">Price Limit</dt>
                  <dd className="font-medium">
                    {quote.price_limit ?? "Market"} {quote.currency}
                  </dd>
                </div>
                <div className="flex justify-between">
                  <dt className="text-muted-foreground">Maker</dt>
                  <dd className="font-mono text-xs">{quote.maker_owner_id}</dd>
                </div>
                <div className="flex justify-between">
                  <dt className="text-muted-foreground">Shard</dt>
                  <dd className="font-mono text-xs">{quote.maker_shard}</dd>
                </div>
              </dl>
            </div>

            <div>
              <h4 className="text-sm font-medium mb-3">Local Law (Guardrails)</h4>
              <dl className="space-y-2 text-sm">
                <div className="flex justify-between">
                  <dt className="text-muted-foreground">Max Debit</dt>
                  <dd className="font-mono text-xs">{formatUnits(quote.local_law.max_debit)}</dd>
                </div>
                <div className="flex justify-between">
                  <dt className="text-muted-foreground">Expires</dt>
                  <dd>{formatUnixDistanceToNow(quote.expires_at)}</dd>
                </div>
                {quote.local_law.allowed_sources.length > 0 && (
                  <div className="flex justify-between">
                    <dt className="text-muted-foreground">Allowed Feeds</dt>
                    <dd className="flex gap-1 flex-wrap">
                      {quote.local_law.allowed_sources.map((s) => (
                        <Badge key={s} variant="outline" className="text-[10px]">
                          {s}
                        </Badge>
                      ))}
                    </dd>
                  </div>
                )}
                <div className="flex justify-between">
                  <dt className="text-muted-foreground">Feed Freshness</dt>
                  <dd>&lt;{quote.local_law.max_staleness_secs}s</dd>
                </div>
                <div className="flex justify-between">
                  <dt className="text-muted-foreground">Quorum</dt>
                  <dd>
                    {quote.local_law.quorum_count} sources, {quote.local_law.quorum_tolerance_percent}% tolerance
                  </dd>
                </div>
                <div className="flex justify-between">
                  <dt className="text-muted-foreground">Atomic DvP</dt>
                  <dd>{quote.local_law.require_atomic_dvp ? "Required" : "No"}</dd>
                </div>
                <div className="flex justify-between">
                  <dt className="text-muted-foreground">Side Payments</dt>
                  <dd>{quote.local_law.no_side_payments ? "Blocked" : "Allowed"}</dd>
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
                    key={receipt.id}
                    className={`p-3 rounded-lg border ${
                      receipt.status === "accepted"
                        ? "border-green-500/50 bg-green-500/10"
                        : "border-red-500/50 bg-red-500/10"
                    }`}
                  >
                    <div className="flex items-center justify-between mb-2">
                      <span className="font-mono text-xs">
                        {receipt.taker_owner_id}
                      </span>
                      <Badge
                        variant={
                          receipt.status === "accepted" ? "success" : "destructive"
                        }
                      >
                        {receipt.status}
                      </Badge>
                    </div>
                    <div className="text-xs text-muted-foreground">
                      Size: {receipt.size} @ {receipt.price} | {formatTimestamp(receipt.attempted_at)}
                    </div>
                    {receipt.status === "rejected" && receipt.rejection_reason && (
                      <div className="mt-2 text-xs text-red-600">
                        Reason: {receipt.rejection_reason}
                      </div>
                    )}
                    {receipt.status === "accepted" && (
                      <div className="mt-3 p-2 bg-green-500/10 rounded text-xs space-y-1">
                        {receipt.sdl_hash && (
                          <div className="flex justify-between">
                            <span className="text-muted-foreground">ZK Proof (SDL):</span>
                            <span className="font-mono text-green-600">{receipt.sdl_hash.slice(0, 20)}...</span>
                          </div>
                        )}
                        {receipt.settlement && (
                          <>
                            <div className="flex justify-between">
                              <span className="text-muted-foreground">Maker Debit:</span>
                              <span className="font-mono">{formatUnits(receipt.settlement.maker_debit)} {receipt.settlement.asset}</span>
                            </div>
                            <div className="flex justify-between">
                              <span className="text-muted-foreground">Maker Credit:</span>
                              <span className="font-mono">{formatUnits(receipt.settlement.maker_credit)} {receipt.settlement.currency}</span>
                            </div>
                            <div className="flex justify-between">
                              <span className="text-muted-foreground">Taker Debit:</span>
                              <span className="font-mono">{formatUnits(receipt.settlement.taker_debit)} {receipt.settlement.currency}</span>
                            </div>
                            <div className="flex justify-between">
                              <span className="text-muted-foreground">Taker Credit:</span>
                              <span className="font-mono">{formatUnits(receipt.settlement.taker_credit)} {receipt.settlement.asset}</span>
                            </div>
                          </>
                        )}
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

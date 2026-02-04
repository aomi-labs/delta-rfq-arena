"use client";

import { Card, CardContent, CardHeader, CardTitle } from "@app/components/ui/card";
import { Badge } from "@app/components/ui/badge";
import type { Quote } from "@app/types/api";
import { formatDistanceToNow } from "@app/lib/format";

interface QuoteCardProps {
  quote: Quote;
  onClick: () => void;
}

export function QuoteCard({ quote, onClick }: QuoteCardProps) {
  const isExpired = new Date(quote.expires_at) < new Date();
  const status = isExpired ? "expired" : quote.status;

  const statusVariant = {
    active: "success" as const,
    filled: "default" as const,
    expired: "warning" as const,
    cancelled: "destructive" as const,
  }[status];

  const sideColor = quote.spec.side === "buy" ? "text-green-600" : "text-red-600";

  return (
    <Card
      className="cursor-pointer transition-all hover:shadow-lg hover:border-primary/50"
      onClick={onClick}
    >
      <CardHeader className="pb-2">
        <div className="flex items-center justify-between">
          <CardTitle className="text-lg">
            <span className={sideColor}>{quote.spec.side.toUpperCase()}</span>{" "}
            {quote.spec.size} {quote.spec.asset}
          </CardTitle>
          <Badge variant={statusVariant}>{status}</Badge>
        </div>
      </CardHeader>
      <CardContent className="space-y-3">
        <div className="grid grid-cols-2 gap-2 text-sm">
          <div>
            <span className="text-muted-foreground">Price:</span>{" "}
            <span className="font-medium">
              {quote.spec.limit_price ? `${quote.spec.limit_price} ${quote.spec.currency}` : "Market"}
            </span>
          </div>
          <div>
            <span className="text-muted-foreground">Maker:</span>{" "}
            <span className="font-mono text-xs">{quote.maker_owner_id.slice(0, 8)}...</span>
          </div>
        </div>

        <div className="text-xs text-muted-foreground space-y-1">
          <div className="flex items-center gap-2">
            <span>Feeds:</span>
            <div className="flex gap-1">
              {quote.constraints.allowed_sources.map((source) => (
                <Badge key={source} variant="outline" className="text-[10px] py-0">
                  {source}
                </Badge>
              ))}
            </div>
          </div>
          <div>
            Freshness: &lt;{quote.constraints.max_staleness_secs}s | Quorum: {quote.constraints.quorum_count}
          </div>
        </div>

        <div className="flex items-center justify-between text-xs pt-2 border-t">
          <span className="text-muted-foreground">
            Expires {formatDistanceToNow(new Date(quote.expires_at))}
          </span>
          {quote.constraints.require_atomic_dvp && (
            <Badge variant="outline" className="text-[10px]">Atomic DvP</Badge>
          )}
        </div>
      </CardContent>
    </Card>
  );
}

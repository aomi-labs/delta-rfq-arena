"use client";

import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { Badge } from "@/components/ui/badge";
import type { Quote } from "@/types/api";
import { formatUnixDistanceToNow, isExpired } from "@/lib/format";

interface QuoteCardProps {
  quote: Quote;
  onClick: () => void;
}

export function QuoteCard({ quote, onClick }: QuoteCardProps) {
  // Guard against incomplete quote data
  if (!quote?.local_law) {
    return null;
  }

  const expired = isExpired(quote.expires_at);
  const status = expired ? "expired" : quote.status;

  const statusVariant = {
    active: "success" as const,
    filled: "default" as const,
    expired: "warning" as const,
    cancelled: "destructive" as const,
  }[status];

  const directionColor = quote.direction === "buy" ? "text-green-600" : "text-red-600";

  return (
    <Card
      className="cursor-pointer transition-all hover:shadow-lg hover:border-primary/50"
      onClick={onClick}
    >
      <CardHeader className="pb-2">
        <div className="flex items-center justify-between">
          <CardTitle className="text-lg">
            <span className={directionColor}>{quote.direction.toUpperCase()}</span>{" "}
            {quote.size} {quote.asset}
          </CardTitle>
          <Badge variant={statusVariant}>{status}</Badge>
        </div>
      </CardHeader>
      <CardContent className="space-y-3">
        <div className="grid grid-cols-2 gap-2 text-sm">
          <div>
            <span className="text-muted-foreground">Price:</span>{" "}
            <span className="font-medium">
              {quote.price_limit ? `${quote.price_limit} ${quote.currency}` : "Market"}
            </span>
          </div>
          <div>
            <span className="text-muted-foreground">Maker:</span>{" "}
            <span className="font-mono text-xs">{quote.maker_owner_id.slice(0, 8)}...</span>
          </div>
        </div>

        <div className="text-xs text-muted-foreground space-y-1">
          {quote.local_law.allowed_sources.length > 0 && (
            <div className="flex items-center gap-2">
              <span>Feeds:</span>
              <div className="flex gap-1">
                {quote.local_law.allowed_sources.map((source) => (
                  <Badge key={source} variant="outline" className="text-[10px] py-0">
                    {source}
                  </Badge>
                ))}
              </div>
            </div>
          )}
          <div>
            Freshness: &lt;{quote.local_law.max_staleness_secs}s | Quorum: {quote.local_law.quorum_count}
          </div>
        </div>

        <div className="flex items-center justify-between text-xs pt-2 border-t">
          <span className="text-muted-foreground">
            Expires {formatUnixDistanceToNow(quote.expires_at)}
          </span>
          {quote.local_law.require_atomic_dvp && (
            <Badge variant="outline" className="text-[10px]">Atomic DvP</Badge>
          )}
        </div>
      </CardContent>
    </Card>
  );
}

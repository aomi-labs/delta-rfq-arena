"use client";

import { useState, useMemo } from "react";
import { Search, RefreshCw } from "lucide-react";
import { Input } from "@/components/ui/input";
import { Button } from "@/components/ui/button";
import { QuoteCard } from "@/components/quote-card";
import { QuoteDetailDialog } from "@/components/quote-detail-dialog";
import type { Quote, FillReceipt } from "@/types/api";

interface QuoteGridProps {
  quotes: Quote[];
  loading: boolean;
  onRefresh: () => void;
  onFetchReceipts: (quoteId: string) => Promise<FillReceipt[]>;
}

export function QuoteGrid({ quotes, loading, onRefresh, onFetchReceipts }: QuoteGridProps) {
  const [search, setSearch] = useState("");
  const [selectedQuote, setSelectedQuote] = useState<Quote | null>(null);
  const [dialogOpen, setDialogOpen] = useState(false);

  const filteredQuotes = useMemo(() => {
    if (!search.trim()) return quotes;
    const lower = search.toLowerCase();
    return quotes.filter(
      (q) =>
        q.spec.asset.toLowerCase().includes(lower) ||
        q.spec.side.toLowerCase().includes(lower) ||
        q.maker_owner_id.toLowerCase().includes(lower) ||
        q.original_text.toLowerCase().includes(lower) ||
        q.status.toLowerCase().includes(lower)
    );
  }, [quotes, search]);

  const handleQuoteClick = (quote: Quote) => {
    setSelectedQuote(quote);
    setDialogOpen(true);
  };

  return (
    <div className="space-y-4">
      {/* Search Bar */}
      <div className="flex items-center gap-3">
        <div className="relative flex-1">
          <Search className="absolute left-3 top-1/2 -translate-y-1/2 h-4 w-4 text-muted-foreground" />
          <Input
            placeholder="Search quotes by asset, side, maker, or status..."
            value={search}
            onChange={(e) => setSearch(e.target.value)}
            className="pl-10"
          />
        </div>
        <Button variant="outline" size="icon" onClick={onRefresh} disabled={loading}>
          <RefreshCw className={`h-4 w-4 ${loading ? "animate-spin" : ""}`} />
        </Button>
      </div>

      {/* Stats */}
      <div className="flex items-center gap-4 text-sm text-muted-foreground">
        <span>{filteredQuotes.length} quotes</span>
        <span>|</span>
        <span className="text-green-600">
          {filteredQuotes.filter((q) => q.status === "active").length} active
        </span>
        <span className="text-blue-600">
          {filteredQuotes.filter((q) => q.status === "filled").length} filled
        </span>
        <span className="text-yellow-600">
          {filteredQuotes.filter((q) => new Date(q.expires_at) < new Date()).length} expired
        </span>
      </div>

      {/* Grid */}
      {filteredQuotes.length === 0 ? (
        <div className="text-center py-12 text-muted-foreground">
          {quotes.length === 0
            ? "No quotes yet. Create one using the Maker panel below."
            : "No quotes match your search."}
        </div>
      ) : (
        <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 xl:grid-cols-4 gap-4">
          {filteredQuotes.map((quote) => (
            <QuoteCard
              key={quote.id}
              quote={quote}
              onClick={() => handleQuoteClick(quote)}
            />
          ))}
        </div>
      )}

      {/* Detail Dialog */}
      <QuoteDetailDialog
        quote={selectedQuote}
        open={dialogOpen}
        onOpenChange={setDialogOpen}
        onFetchReceipts={onFetchReceipts}
      />
    </div>
  );
}

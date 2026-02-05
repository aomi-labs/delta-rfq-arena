"use client";

import { QuoteGrid } from "@/components/quote-grid";
import dynamic from "next/dynamic";
const AomiFrameWrapper = dynamic(
  () => import("@/components/agent-panel").then((mod) => mod.AomiFrameWrapper),
  { ssr: false },
);
import { useQuotes } from "@/hooks/use-quotes";
import { Badge } from "@/components/ui/badge";

export default function Home() {
  const { quotes, loading, refresh, getReceipts } = useQuotes();

  return (
    <div className="min-h-screen flex flex-col">
      {/* Header */}
      <header className="border-b bg-card">
        <div className="container mx-auto px-4 py-4">
          <div className="flex items-center justify-between">
            <div className="flex items-center gap-3">
              <h1 className="text-2xl font-bold">RFQ Arena</h1>
              <Badge variant="outline" className="text-xs">
                Delta Network
              </Badge>
            </div>
            <div className="flex items-center gap-4 text-sm text-muted-foreground">
              <div className="flex items-center gap-2">
                <div className="w-2 h-2 rounded-full bg-green-500 animate-pulse" />
                <span>Testnet</span>
              </div>
              <span className="font-mono">Shard 9</span>
            </div>
          </div>
        </div>
      </header>

      {/* Main Content */}
      <main className="flex-1 container mx-auto px-4 py-6 space-y-8">
        {/* Section 1: Quote Market */}
        <section>
          <div className="flex items-center gap-3 mb-4">
            <h2 className="text-xl font-semibold">Quote Market</h2>
            <p className="text-sm text-muted-foreground">
              Browse active RFQ quotes with ZK-enforced guardrails
            </p>
          </div>
          <QuoteGrid
            quotes={quotes}
            loading={loading}
            onRefresh={refresh}
            onFetchReceipts={getReceipts}
          />
        </section>

        {/* Section 2: Agent Panels */}
        <section>
          <div className="flex items-center gap-3 mb-4">
            <h2 className="text-xl font-semibold">Trading Agents</h2>
            <p className="text-sm text-muted-foreground">
              Chat with AI agents to create and fill quotes
            </p>
          </div>
          <div className="grid grid-cols-1 lg:grid-cols-2 gap-6 h-[800px]">
            {/* Maker Panel */}
            <div className="flex flex-col min-h-0">
              <div className="flex items-center gap-2 mb-2 shrink-0">
                <div className="w-3 h-3 rounded-full bg-green-500" />
                <span className="text-sm font-medium">Maker Agent</span>
                <span className="text-xs text-muted-foreground">
                  Create quotes in plain English
                </span>
              </div>
              <div className="flex-1 min-h-0 overflow-hidden rounded-2xl border shadow-lg">
                <AomiFrameWrapper role="maker" />
              </div>
            </div>

            {/* Taker Panel */}
            <div className="flex flex-col min-h-0">
              <div className="flex items-center gap-2 mb-2 shrink-0">
                <div className="w-3 h-3 rounded-full bg-blue-500" />
                <span className="text-sm font-medium">Taker Agent</span>
                <span className="text-xs text-muted-foreground">
                  Find and fill quotes
                </span>
              </div>
              <div className="flex-1 min-h-0 overflow-hidden rounded-2xl border shadow-lg">
                <AomiFrameWrapper role="taker" />
              </div>
            </div>
          </div>
        </section>

        {/* Info Section */}
        <section className="border-t pt-6">
          <div className="grid grid-cols-1 md:grid-cols-3 gap-6 text-sm">
            <div className="space-y-2">
              <h3 className="font-semibold">How It Works</h3>
              <ol className="list-decimal list-inside text-muted-foreground space-y-1">
                <li>Maker describes quote in English</li>
                <li>System compiles to guardrails</li>
                <li>Taker attempts to fill</li>
                <li>Local Laws validate on-chain</li>
              </ol>
            </div>
            <div className="space-y-2">
              <h3 className="font-semibold">Guardrails Enforced</h3>
              <ul className="text-muted-foreground space-y-1">
                <li>Feed freshness &amp; source allowlist</li>
                <li>Price quorum requirements</li>
                <li>Atomic DvP settlement</li>
                <li>No side-payments</li>
              </ul>
            </div>
            <div className="space-y-2">
              <h3 className="font-semibold">Attacks Blocked</h3>
              <ul className="text-muted-foreground space-y-1">
                <li>Stale feed manipulation</li>
                <li>Spoofed price sources</li>
                <li>Overfill / replay attacks</li>
                <li>Hidden skim attempts</li>
              </ul>
            </div>
          </div>
        </section>
      </main>

      {/* Footer */}
      <footer className="border-t py-4 text-center text-sm text-muted-foreground">
        <p>
          Powered by{" "}
          <a
            href="https://docs.repyhlabs.dev/"
            target="_blank"
            rel="noopener noreferrer"
            className="font-medium text-foreground hover:underline"
          >
            Delta Network
          </a>{" "}
          &amp;{" "}
          <a
            href="https://aomi.ai"
            target="_blank"
            rel="noopener noreferrer"
            className="font-medium text-foreground hover:underline"
          >
            Aomi Runtime
          </a>
        </p>
      </footer>
    </div>
  );
}

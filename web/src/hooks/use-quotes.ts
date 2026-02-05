"use client";

import { useState, useEffect, useCallback } from "react";
import type { Quote, FillReceipt, CreateQuoteRequest, CreateQuoteResponse, FillRequest, FillResponse } from "@/types/api";

const API_BASE = process.env.NEXT_PUBLIC_API_URL || "http://localhost:3335";

export function useQuotes() {
  const [quotes, setQuotes] = useState<Quote[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  const fetchQuotes = useCallback(async () => {
    try {
      setLoading(true);
      const res = await fetch(`${API_BASE}/quotes`);
      if (!res.ok) throw new Error("Failed to fetch quotes");
      const data = await res.json();
      setQuotes(data);
      setError(null);
    } catch (err) {
      setError(err instanceof Error ? err.message : "Unknown error");
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    fetchQuotes();
    // Poll every 5 seconds
    const interval = setInterval(fetchQuotes, 5000);
    return () => clearInterval(interval);
  }, [fetchQuotes]);

  const createQuote = async (request: CreateQuoteRequest): Promise<CreateQuoteResponse> => {
    const res = await fetch(`${API_BASE}/quotes`, {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify(request),
    });
    if (!res.ok) {
      const text = await res.text();
      throw new Error(text || "Failed to create quote");
    }
    const data = await res.json();
    await fetchQuotes();
    return data;
  };

  const fillQuote = async (quoteId: string, request: FillRequest): Promise<FillResponse> => {
    const res = await fetch(`${API_BASE}/quotes/${quoteId}/fill`, {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify(request),
    });
    if (!res.ok) {
      const text = await res.text();
      throw new Error(text || "Failed to fill quote");
    }
    const data = await res.json();
    await fetchQuotes();
    return data;
  };

  const getReceipts = async (quoteId: string): Promise<FillReceipt[]> => {
    const res = await fetch(`${API_BASE}/quotes/${quoteId}/receipts`);
    if (!res.ok) throw new Error("Failed to fetch receipts");
    return res.json();
  };

  return {
    quotes,
    loading,
    error,
    refresh: fetchQuotes,
    createQuote,
    fillQuote,
    getReceipts,
  };
}

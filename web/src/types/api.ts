// API Types matching the Rust backend

export type Side = "buy" | "sell";

export type QuoteStatus = "active" | "filled" | "expired" | "cancelled";

export interface QuoteSpec {
  asset: string;
  size: number;
  side: Side;
  limit_price: number | null;
  currency: string;
}

export interface QuoteConstraints {
  quote_id: number[];
  max_debit: number;
  min_credit: number | null;
  expiry_timestamp: number;
  allowed_sources: string[];
  max_staleness_secs: number;
  quorum_count: number;
  quorum_tolerance_percent: number;
  allowed_takers: string[];
  allowed_assets: string[];
  require_atomic_dvp: boolean;
  no_side_payments: boolean;
  nonce: number;
  max_fill_size: number;
}

export interface Quote {
  id: string;
  spec: QuoteSpec;
  constraints: QuoteConstraints;
  status: QuoteStatus;
  created_at: string;
  expires_at: string;
  maker_owner_id: string;
  maker_vault_address: string;
  original_text: string;
}

export interface FeedEvidence {
  source: string;
  asset: string;
  price: number;
  timestamp: number;
  signature: string;
}

export interface FillRequest {
  taker_owner_id: string;
  taker_shard: number;
  size: number;
  price: number;
  feed_evidence: FeedEvidence[];
}

export interface SettlementDetails {
  maker_debit: number;
  maker_credit: number;
  taker_debit: number;
  taker_credit: number;
  asset: string;
  currency: string;
  settled_at: string;
}

export interface RejectionReason {
  code: string;
  [key: string]: unknown;
}

export type FillResult =
  | {
      status: "accepted";
      fill_id: string;
      sdl_hash: string;
      settlement: SettlementDetails;
    }
  | {
      status: "rejected";
      fill_id: string;
      reason: RejectionReason;
    };

export interface FillAttempt {
  id: string;
  quote_id: string;
  taker_owner_id: string;
  taker_shard: number;
  size: number;
  price: number;
  feed_evidence: FeedEvidence[];
  attempted_at: string;
}

export interface FillReceipt {
  id: string;
  quote: Quote;
  constraints: QuoteConstraints;
  fill_attempt: FillAttempt;
  result: FillResult;
  created_at: string;
}

export interface CreateQuoteRequest {
  text: string;
  maker_owner_id: string;
  maker_shard: number;
}

export interface CreateQuoteResponse {
  quote: Quote;
  constraints_summary: string;
}

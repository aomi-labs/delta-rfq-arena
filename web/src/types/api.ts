// API Types matching the actual Rust backend responses

export type Direction = "buy" | "sell";

export type QuoteStatus = "active" | "filled" | "expired" | "cancelled";

// Local Law constraints from the backend
export interface LocalLaw {
  max_debit: number;
  expiry_timestamp: number;
  allowed_sources: string[];
  max_staleness_secs: number;
  quorum_count: number;
  quorum_tolerance_percent: number;
  require_atomic_dvp: boolean;
  no_side_payments: boolean;
}

// Quote as returned by GET /quotes and GET /quotes/:id
export interface Quote {
  id: string;
  text: string;
  status: QuoteStatus;
  asset: string;
  direction: Direction;
  size: number;
  price_limit: number | null;
  currency: string;
  expires_at: number; // Unix timestamp
  created_at: number; // Unix timestamp
  maker_owner_id: string;
  maker_shard: number;
  local_law: LocalLaw;
}

// Request to create a new quote
export interface CreateQuoteRequest {
  text: string;
  maker_owner_id: string;
  maker_shard: number;
}

// Response from POST /quotes
export interface CreateQuoteResponse extends Quote {
  constraints_summary: string;
  message: string;
}

// Feed evidence for fill requests
export interface FeedEvidence {
  source: string;
  asset: string;
  price: number;
  timestamp: number;
  signature: string;
}

// Request to fill a quote
export interface FillRequest {
  taker_owner_id: string;
  taker_shard: number;
  size: number;
  price: number;
  feed_evidence: FeedEvidence[];
}

// Settlement details in fill response
export interface Settlement {
  maker_debit: number;
  maker_credit: number;
  taker_debit: number;
  taker_credit: number;
  asset: string;
  currency: string;
}

// Receipt in fill response
export interface FillResponseReceipt {
  id: string;
  quote_id: string;
  taker_owner_id: string;
  taker_shard: number;
  size: number;
  price: number;
  filled_at: number; // Unix timestamp
  settlement: Settlement;
}

// Proof info in fill response
export interface Proof {
  sdl_hash: string;
  status: string;
}

// Response from POST /quotes/:id/fill
export interface FillResponse {
  success: boolean;
  fill_id: string;
  quote_id: string;
  message: string;
  receipt: FillResponseReceipt;
  proof: Proof;
}

// Receipt as returned by GET /quotes/:id/receipts
export interface FillReceipt {
  id: string;
  quote_id: string;
  success: boolean;
  status: "accepted" | "rejected";
  taker_owner_id: string;
  taker_shard: number;
  size: number;
  price: number;
  attempted_at: number; // Unix timestamp
  sdl_hash?: string;
  rejection_reason?: string;
  error_code?: string;
  error_message?: string;
  settlement?: Settlement;
}

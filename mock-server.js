#!/usr/bin/env node
// Simple mock RFQ server for development
const http = require('http');

const quotes = new Map();
const receipts = new Map();
let nextId = 1;

function parseQuoteText(text) {
  const lower = text.toLowerCase();
  const direction = lower.includes('buy') ? 'buy' : 'sell';
  const asset = lower.includes('dbtc') || lower.includes('btc') ? 'dBTC' : 'dETH';
  
  // Extract size (number after buy/sell)
  const sizeMatch = lower.match(/(?:buy|sell)\s+(\d+(?:\.\d+)?)/);
  const size = sizeMatch ? parseFloat(sizeMatch[1]) : 1;
  
  // Extract price limit
  const priceMatch = lower.match(/(?:at most|at least|for)\s+(\d+(?:\.\d+)?)/);
  const priceLimit = priceMatch ? parseFloat(priceMatch[1]) : 2000;
  
  return { direction, asset, size, priceLimit };
}

function handleRequest(req, res) {
  const url = new URL(req.url, `http://${req.headers.host}`);
  
  // CORS
  res.setHeader('Access-Control-Allow-Origin', '*');
  res.setHeader('Access-Control-Allow-Methods', 'GET, POST, OPTIONS');
  res.setHeader('Access-Control-Allow-Headers', 'Content-Type');
  
  if (req.method === 'OPTIONS') {
    res.writeHead(204);
    return res.end();
  }
  
  // Health check
  if (url.pathname === '/health') {
    res.writeHead(200, { 'Content-Type': 'application/json' });
    return res.end(JSON.stringify({ status: 'ok' }));
  }
  
  // GET /quotes - list all quotes
  if (url.pathname === '/quotes' && req.method === 'GET') {
    const all = Array.from(quotes.values());
    res.writeHead(200, { 'Content-Type': 'application/json' });
    return res.end(JSON.stringify(all));
  }
  
  // POST /quotes - create quote
  if (url.pathname === '/quotes' && req.method === 'POST') {
    let body = '';
    req.on('data', chunk => body += chunk);
    req.on('end', () => {
      try {
        const data = JSON.parse(body);
        const { direction, asset, size, priceLimit } = parseQuoteText(data.text || '');
        
        const quote = {
          id: `quote_${nextId++}`,
          text: data.text,
          status: 'active',
          asset,
          direction,
          size,
          price_limit: priceLimit,
          currency: 'USDD',
          maker_owner_id: data.maker_owner_id || 'anon',
          maker_shard: data.maker_shard || 9,
          expires_at: new Date(Date.now() + 5 * 60 * 1000).toISOString(),
          created_at: new Date().toISOString(),
          local_law: {
            version: '1.0',
            constraints: [
              { type: 'asset_check', asset },
              { type: 'direction_check', direction },
              { type: 'size_check', max_size: size },
              { type: 'price_check', operator: direction === 'buy' ? '<=' : '>=', limit: priceLimit }
            ]
          }
        };
        
        quotes.set(quote.id, quote);
        receipts.set(quote.id, []);
        
        res.writeHead(200, { 'Content-Type': 'application/json' });
        res.end(JSON.stringify({ quote }));
      } catch (e) {
        res.writeHead(400, { 'Content-Type': 'application/json' });
        res.end(JSON.stringify({ error: e.message }));
      }
    });
    return;
  }
  
  // GET /quotes/:id
  const getMatch = url.pathname.match(/^\/quotes\/([^\/]+)$/);
  if (getMatch && req.method === 'GET') {
    const quote = quotes.get(getMatch[1]);
    if (!quote) {
      res.writeHead(404, { 'Content-Type': 'application/json' });
      return res.end(JSON.stringify({ error: 'Quote not found' }));
    }
    res.writeHead(200, { 'Content-Type': 'application/json' });
    return res.end(JSON.stringify(quote));
  }
  
  // POST /quotes/:id/fill
  const fillMatch = url.pathname.match(/^\/quotes\/([^\/]+)\/fill$/);
  if (fillMatch && req.method === 'POST') {
    const quote = quotes.get(fillMatch[1]);
    if (!quote) {
      res.writeHead(404, { 'Content-Type': 'application/json' });
      return res.end(JSON.stringify({ error: 'Quote not found' }));
    }
    
    let body = '';
    req.on('data', chunk => body += chunk);
    req.on('end', () => {
      try {
        const data = JSON.parse(body);
        
        // Validate fill
        if (quote.direction === 'buy' && data.price > quote.price_limit) {
          res.writeHead(200, { 'Content-Type': 'application/json' });
          return res.end(JSON.stringify({
            result: { status: 'rejected', reason: { code: 'price_exceeds_limit', fill_price: data.price, limit: quote.price_limit }}
          }));
        }
        if (quote.direction === 'sell' && data.price < quote.price_limit) {
          res.writeHead(200, { 'Content-Type': 'application/json' });
          return res.end(JSON.stringify({
            result: { status: 'rejected', reason: { code: 'price_below_limit', fill_price: data.price, limit: quote.price_limit }}
          }));
        }
        
        // Create receipt
        const receipt = {
          id: `receipt_${Date.now()}`,
          quote_id: quote.id,
          taker_owner_id: data.taker_owner_id,
          taker_shard: data.taker_shard,
          size: data.size,
          price: data.price,
          filled_at: new Date().toISOString(),
          tx_hash: `0x${Math.random().toString(16).slice(2)}`,
          proof: 'mock_zk_proof'
        };
        
        receipts.get(quote.id).push(receipt);
        quote.status = 'filled';
        
        res.writeHead(200, { 'Content-Type': 'application/json' });
        res.end(JSON.stringify({ result: { status: 'filled', receipt, proof: 'mock_zk_proof' }}));
      } catch (e) {
        res.writeHead(400, { 'Content-Type': 'application/json' });
        res.end(JSON.stringify({ error: e.message }));
      }
    });
    return;
  }
  
  // GET /quotes/:id/receipts
  const receiptsMatch = url.pathname.match(/^\/quotes\/([^\/]+)\/receipts$/);
  if (receiptsMatch && req.method === 'GET') {
    const r = receipts.get(receiptsMatch[1]) || [];
    res.writeHead(200, { 'Content-Type': 'application/json' });
    return res.end(JSON.stringify(r));
  }
  
  res.writeHead(404, { 'Content-Type': 'application/json' });
  res.end(JSON.stringify({ error: 'Not found' }));
}

const PORT = process.env.API_PORT || 3335;
const server = http.createServer(handleRequest);
server.listen(PORT, () => {
  console.log(`🌊 Mock RFQ Server running on http://localhost:${PORT}`);
  console.log('Endpoints: GET/POST /quotes, GET /quotes/:id, POST /quotes/:id/fill, GET /quotes/:id/receipts');
});

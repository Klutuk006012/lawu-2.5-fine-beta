// ============================================================
// k6-loadtest.js — Load test Lawu-2.5-Fine (Langkah 16)
// ------------------------------------------------------------
// Target: p(95) < 300ms, error rate < 1%, dengan 20 VUs selama 60s
// Test: GET /api/corpus/search?q=... (pencarian Tantivy paling berat)
// Jalankan setelah install k6:
//   k6 run --vus 20 --duration 60s k6-loadtest.js
// ============================================================
import http from 'k6/http';
import { check, sleep } from 'k6';
import { Rate } from 'k6/metrics';

// Custom metric: error rate
const errorRate = new Rate('errors');

// Konfigurasi default (bisa di-override via CLI flag --vus, --duration)
export const options = {
  stages: [
    { duration: '10s', target: 20 },  // ramp-up 0→20 VUs dalam 10s
    { duration: '40s', target: 20 },  // hold 20 VUs selama 40s
    { duration: '10s', target: 0 },   // ramp-down 20→0 VUs dalam 10s
  ],
  thresholds: {
    // SLO Langkah 16: p95 < 300ms, error rate < 1%
    http_req_duration: ['p(95)<300'],
    errors: ['rate<0.01'],
  },
};

// Pool query pencarian (acak per request supaya cache tidak mendominasi)
const QUERIES = [
  'google', 'ai agents', 'rag', 'prompting', 'keamanan siber',
  'onno purbo', 'rust programming', 'vue.js', 'wikipedia indonesia',
  'cyber warfare', 'data pribadi', 'llm', 'knowledge graph',
  'postgresql', 'algorithmic thinking', 'machine learning',
  'pencarian google', 'open source', 'cyber security', 'tantivy',
];

export default function () {
  // Pilih query acak dari pool
  const q = QUERIES[Math.floor(Math.random() * QUERIES.length)];
  const url = `http://127.0.0.1:80/api/corpus/web?q=${encodeURIComponent(q)}`;

  const res = http.get(url, {
    headers: { 'Accept': 'application/json' },
    tags: { endpoint: 'corpus_search' },
  });

  // Cek: harus 200 + body JSON valid
  const ok = check(res, {
    'status 200': (r) => r.status === 200,
    'body ada': (r) => r.body && r.body.length > 10,
  });

  errorRate.add(!ok);

  // Jeda singkat antar request (simulasi user think time)
  sleep(0.5 + Math.random() * 0.5);
}

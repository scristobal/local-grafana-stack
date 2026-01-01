import http from 'k6/http';
import { sleep, check } from 'k6';
import { Rate, Trend, Counter } from 'k6/metrics';

const errorRate = new Rate('errors');
const requestDuration = new Trend('request_duration');
const requestCount = new Counter('requests');

export const options = {
  stages: [
    { duration: '30s', target: 10 },  // Ramp up to 10 users
    { duration: '1m', target: 10 },   // Stay at 10 users
    { duration: '30s', target: 20 },  // Ramp up to 20 users
    { duration: '1m', target: 20 },   // Stay at 20 users
    { duration: '30s', target: 0 },   // Ramp down to 0
  ],
  thresholds: {
    http_req_duration: ['p(95)<500', 'p(99)<1000'],
    http_req_failed: ['rate<0.1'],
    errors: ['rate<0.1'],
  },
};

const BASE_URL = __ENV.BASE_URL || 'http://rust-app:8080';

export default function () {
  const requests = [
    { method: 'GET', url: `${BASE_URL}/health` },
    { method: 'GET', url: `${BASE_URL}/user/${Math.floor(Math.random() * 100)}` },
    {
      method: 'POST',
      url: `${BASE_URL}/calculate/add`,
      body: JSON.stringify({
        a: Math.random() * 100,
        b: Math.random() * 100,
      }),
      params: { headers: { 'Content-Type': 'application/json' } },
    },
    {
      method: 'POST',
      url: `${BASE_URL}/calculate/divide`,
      body: JSON.stringify({
        a: Math.random() * 100,
        b: Math.floor(Math.random() * 10) + 1, // Avoid divide by zero most of the time
      }),
      params: { headers: { 'Content-Type': 'application/json' } },
    },
  ];

  const req = requests[Math.floor(Math.random() * requests.length)];
  const startTime = Date.now();

  let response;
  if (req.method === 'POST') {
    response = http.post(req.url, req.body, req.params);
  } else {
    response = http.get(req.url);
  }

  const duration = Date.now() - startTime;
  requestDuration.add(duration);
  requestCount.add(1);

  const success = check(response, {
    'status is 200 or 400': (r) => r.status === 200 || r.status === 400,
    'response has body': (r) => r.body.length > 0,
  });

  if (!success) {
    errorRate.add(1);
  } else {
    errorRate.add(0);
  }

  sleep(Math.random() * 2 + 0.5); // Random sleep between 0.5 and 2.5 seconds
}

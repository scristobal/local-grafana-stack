import http from 'k6/http';
import { sleep, check } from 'k6';
import { Counter } from 'k6/metrics';

const errors = new Counter('intentional_errors');

// Generates errors to validate error handling, logging, and alerting
export const options = {
  stages: [
    { duration: '30s', target: 10 },
    { duration: '2m', target: 10 },
    { duration: '30s', target: 0 },
  ],
};

const BASE_URL = __ENV.BASE_URL || 'http://rust-app:8080';

export default function () {
  // Mix of successful and error-generating requests
  const requests = [
    // These should succeed
    { url: `${BASE_URL}/health`, shouldError: false },
    {
      url: `${BASE_URL}/calculate/add`,
      method: 'POST',
      body: JSON.stringify({ a: 10, b: 20 }),
      shouldError: false,
    },
    // These should cause errors
    {
      url: `${BASE_URL}/calculate/divide`,
      method: 'POST',
      body: JSON.stringify({ a: 100, b: 0 }), // Division by zero
      shouldError: true,
    },
    {
      url: `${BASE_URL}/simulate/error`,
      shouldError: true,
    },
  ];

  const req = requests[Math.floor(Math.random() * requests.length)];

  let response;
  if (req.method === 'POST') {
    response = http.post(req.url, req.body, {
      headers: { 'Content-Type': 'application/json' },
    });
  } else {
    response = http.get(req.url);
  }

  if (req.shouldError) {
    check(response, {
      'expected error status': (r) => r.status >= 400,
    });
    errors.add(1);
  } else {
    check(response, {
      'success status': (r) => r.status === 200,
    });
  }

  sleep(Math.random() * 1.5 + 0.5);
}

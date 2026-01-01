import http from 'k6/http';
import { sleep, check } from 'k6';
import { Rate } from 'k6/metrics';

const errorRate = new Rate('errors');

export const options = {
  stages: [
    { duration: '1m', target: 50 },   // Ramp up to 50 users
    { duration: '2m', target: 100 },  // Ramp up to 100 users
    { duration: '3m', target: 100 },  // Stay at 100 users
    { duration: '1m', target: 150 },  // Push to 150 users
    { duration: '2m', target: 150 },  // Stay at 150 users
    { duration: '1m', target: 0 },    // Ramp down
  ],
};

const BASE_URL = __ENV.BASE_URL || 'http://rust-app:8080';

export default function () {
  const endpoints = [
    `${BASE_URL}/health`,
    `${BASE_URL}/user/${Math.floor(Math.random() * 1000)}`,
  ];

  const response = http.get(endpoints[Math.floor(Math.random() * endpoints.length)]);

  const success = check(response, {
    'status is 200': (r) => r.status === 200,
  });

  errorRate.add(!success);

  sleep(0.5);
}

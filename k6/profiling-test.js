import http from 'k6/http';
import { sleep } from 'k6';

// Uses slow endpoint to generate CPU load for profiling
export const options = {
  stages: [
    { duration: '30s', target: 5 },
    { duration: '2m', target: 5 },
    { duration: '30s', target: 0 },
  ],
};

const BASE_URL = __ENV.BASE_URL || 'http://rust-app:8080';

export default function () {
  const operations = [
    () => http.get(`${BASE_URL}/health`),
    () => http.get(`${BASE_URL}/user/${Math.floor(Math.random() * 100)}`),
    () => http.get(`${BASE_URL}/simulate/slow`), // 2 second delay
    () =>
      http.post(
        `${BASE_URL}/calculate/add`,
        JSON.stringify({
          a: Math.random() * 1000,
          b: Math.random() * 1000,
        }),
        { headers: { 'Content-Type': 'application/json' } }
      ),
  ];

  operations[Math.floor(Math.random() * operations.length)]();

  sleep(1);
}

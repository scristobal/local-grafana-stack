import http from 'k6/http';
import { sleep, check } from 'k6';

export const options = {
  stages: [
    { duration: '30s', target: 10 },   // Normal load
    { duration: '10s', target: 200 },  // Sudden spike!
    { duration: '1m', target: 200 },   // Stay at spike
    { duration: '10s', target: 10 },   // Back to normal
    { duration: '30s', target: 10 },   // Normal load
    { duration: '10s', target: 300 },  // Another spike!
    { duration: '30s', target: 300 },  // Brief spike
    { duration: '10s', target: 0 },    // Ramp down
  ],
};

const BASE_URL = __ENV.BASE_URL || 'http://rust-app:8080';

export default function () {
  const response = http.get(`${BASE_URL}/health`);

  check(response, {
    'status is 200': (r) => r.status === 200,
  });

  sleep(0.3);
}

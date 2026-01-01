import http from 'k6/http';
import { sleep, check } from 'k6';

// Extended run to detect memory leaks and performance degradation over time
export const options = {
  stages: [
    { duration: '2m', target: 20 },    // Ramp up
    { duration: '10m', target: 20 },   // Stay steady for 10 minutes
    { duration: '1m', target: 0 },     // Ramp down
  ],
};

const BASE_URL = __ENV.BASE_URL || 'http://rust-app:8080';

export default function () {
  const responses = http.batch([
    ['GET', `${BASE_URL}/health`],
    ['GET', `${BASE_URL}/user/${Math.floor(Math.random() * 100)}`],
  ]);

  responses.forEach((response) => {
    check(response, {
      'status is 200': (r) => r.status === 200,
    });
  });

  sleep(1);
}

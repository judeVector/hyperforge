import http from "k6/http";
import { check } from "k6";

export const options = {
  // vus: 10,
  // iterations: 40,

  discardResponseBodies: true,
  thresholds: {
    http_req_failed: ["rate<0.01"],
    http_req_duration: ["p(95)<500"],
  },
  stages: [
    { duration: "2m", target: 200 },
    { duration: "3m", target: 500 },
    { duration: "3m", target: 1000 },
    { duration: "3m", target: 1500 },
    { duration: "3m", target: 2000 },
    { duration: "2m", target: 0 },
  ],
};

export default function () {
  const res = http.get("http://localhost:3000/users");

  check(res, {
    "status is 200": (r) => r.status === 200,
  });
}

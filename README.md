## Tower-Server

A lightweight **Rust HTTP API** built with **Hyper**, **Tower**, and **SQLx**.  
This project demonstrates how to build a production-style async server **without high-level frameworks**.

## Features

- Async HTTP/1 server using **Hyper**
- Middleware with **Tower** (CORS, timeout, concurrency limits, tracing)
- PostgreSQL integration via **SQLx**
- Basic User CRUD API
- Health check with DB validation
- In-memory metrics endpoint
- Graceful shutdown (CTRL+C)

## Tech Stack

- Rust + Tokio
- Hyper & Tower
- SQLx (PostgreSQL)
- Tracing

## Getting Started

### Prerequisites
- Rust (stable)
- PostgreSQL

### Setup

```bash
git clone https://github.com/judevector/tower-server.git
cd tower-server
```

Create a `.env` file:
```env
DATABASE_URL=postgresql://postgres:password@localhost:5432/postgres
```

Run the server:
```env
cargo run
```

Server runs on:
```env
http://127.0.0.1:3000
```

API Endpoints

- GET /health – Service & database health
- GET /users – Fetch all users
- GET /users/{id} – Fetch user by ID
- POST /users – Create a new user
- DELETE /users/{id} – Delete a user
- GET /metrics – Request & error counts

## Notes
Request timeout: 30s

Max concurrency: 100

Payload limit: 64KB

Database schema is created automatically on startup

## License

MIT

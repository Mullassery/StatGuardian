# StatGuardian service deployment

```bash
cd deploy
docker compose up --build
curl -s http://127.0.0.1:8000/healthz
```

Then point PyReverseETL at it:

```bash
pyreverseetl execute-activation my-activation \
  --statguardian-url http://127.0.0.1:8000 \
  --statguardian-limit 100
```

Notes:

- The image builds the Rust extension in a multi-stage Dockerfile.
- The sample contract is `deploy/contracts/orders.sg`; replace it or mount your
  own over `/contracts/orders.sg`.
- The compose port binds to loopback only. Do not expose this service publicly
  without adding authentication and TLS.
- `/detect-changes` intentionally returns 501.

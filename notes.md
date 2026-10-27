# Notes

Commands I might need. Server commands run from `web/` unless stated otherwise.

## Local development

```bash
cargo run                   # dev server on http://localhost:3000
cargo test                  # unit tests (services.rs, time_range.rs)
RUST_LOG=debug cargo run    # more logging
```

`cargo run` checks queries against the live `data.db` (from `DATABASE_URL` in `.env`). On a fresh clone that file doesn't exist yet, so use `SQLX_OFFLINE=true` for the first build.

## Release build

Same as CI:

```bash
SQLX_OFFLINE=true cargo build --release --locked
```

Binary: `target/release/web`. It serves `static/` relative to the directory it runs in.

Package it like the deploy workflow does:

```bash
tar -czf deploy.tar.gz -C target/release web -C ../.. static
```

## SQLx

Install the CLI once:

```bash
cargo install sqlx-cli --no-default-features --features sqlite
```

Regenerate `.sqlx/` after changing any `query!` / `query_as!` (needs a migrated `data.db`), then commit it:

```bash
cargo sqlx prepare
cargo sqlx prepare --check  # verify .sqlx/ is up to date
```

Migrations:

```bash
sqlx migrate add <name>     # new file in migrations/
sqlx migrate run            # apply to data.db (the server also runs them on start)
sqlx database create        # create an empty data.db
```



## Testing the API

```bash
# Fake beat; use a test device_id so it doesn't mix with real data
curl -i -X POST http://localhost:3000/beat \
  -H "Authorization: Bearer $API_KEY" \
  -H "Content-Type: application/json" \
  -d '{"device_id":"test","boot_id":1,"reset_reason":"power_on","uptime_ms":4000}'

curl http://localhost:3000/summary/homev1
curl "http://localhost:3000/period/homev1?filter=today"
curl http://localhost:3000/health
```

## Database

```bash
sqlite3 data.db "SELECT * FROM power_periods ORDER BY started_at DESC LIMIT 10;"
sqlite3 data.db "DELETE FROM power_periods WHERE device_id = 'test';"
sqlite3 data.db ".backup data.db.bak"   # safe while the server is running (WAL)
```

## Deploying

A push to `main` builds and deploys automatically. To redeploy without a new commit, use **Actions → Deploy → Run workflow** on GitHub.

## On the VPS

The app lives in `~/electricity-monitor`, run by the `electricity-monitor` systemd unit.

```bash
systemctl status electricity-monitor
sudo systemctl restart electricity-monitor
journalctl -u electricity-monitor -f          # follow logs
journalctl -u electricity-monitor -n 100      # last 100 lines
```

Restore the backup the deploy takes before each restart:

```bash
cd ~/electricity-monitor
sudo systemctl stop electricity-monitor
cp data.db.bak data.db && rm -f data.db-wal data.db-shm
sudo systemctl start electricity-monitor
```

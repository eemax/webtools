# webtools

Fast web primitives for agent harnesses.

The CLI is intentionally small:

- `search` uses Exa and returns normalized JSON.
- `fetch` performs a simple native HTTP fetch and returns Markdown content in JSON.
- JSON is the default output because harnesses are the primary caller.
- `fetch --md` prints only the Markdown body for quick human inspection.

## Usage

Build locally:

```bash
cargo build --release
```

Install from the checkout:

```bash
cargo install --path .
```

```bash
webtools search "rust async runtime comparison"
webtools search --count 10 "openai responses api web search"
webtools search -n 3 --type neural "rust html parser"

webtools fetch https://example.com
webtools fetch --md https://example.com
webtools --version
```

`search` requires `EXA_API_KEY`.

## Development

Run the standard local gate:

```bash
just check
```

Or without `just`:

```bash
cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo test
```

Live tests are ignored by default:

```bash
cargo test --test live -- --ignored
```

Run a tiny live fetch smoke benchmark:

```bash
scripts/bench_fetch.sh
```

## Shape

```json
{
  "ok": true,
  "provider": "exa",
  "query": "rust async runtime comparison",
  "type": "auto",
  "results": [
    {
      "title": "Example",
      "url": "https://example.com",
      "published_date": null,
      "score": 0.42,
      "highlights": []
    }
  ]
}
```

```json
{
  "ok": true,
  "url": "https://example.com",
  "final_url": "https://example.com/",
  "status": 200,
  "content_type": "text/html",
  "title": "Example Domain",
  "kind": "html",
  "content": "# Example Domain\n\n...",
  "warnings": [],
  "truncated": false,
  "bytes_read": 1256,
  "elapsed_ms": 184,
  "error": null
}
```

## Exit Policy

`webtools` treats ordinary fetch failures as data, not process failures.

- Usage errors, bad flags, and runtime failures such as missing `EXA_API_KEY` exit nonzero.
- `fetch` failures like `invalid_url`, `blocked_host`, HTTP 404, or transport errors exit zero and return JSON with `"ok": false`.
- `fetch --md` prints only the Markdown body. If the fetch result is not ok, the body is empty.

## Non-goals

- no browser automation
- no crawler
- no JavaScript rendering
- no robots.txt crawling policy; this is a single-URL fetch tool, not a crawler
- no enterprise-grade network stack
- no runtime Defuddle/Node dependency

The fetcher should be excellent on common docs, blogs, changelogs, articles, raw files, and JSON endpoints. Weird pages can fail cheaply and clearly.

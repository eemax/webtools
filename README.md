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

webtools fetch https://example.com
webtools fetch --md https://example.com
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

## Shape

```json
{
  "ok": true,
  "provider": "exa",
  "query": "rust async runtime comparison",
  "type": "auto",
  "results": []
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
- no enterprise-grade network stack
- no runtime Defuddle/Node dependency

The fetcher should be excellent on common docs, blogs, changelogs, articles, raw files, and JSON endpoints. Weird pages can fail cheaply and clearly.

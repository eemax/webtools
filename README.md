# webtools

Fast web primitives for agent harnesses.

The CLI is intentionally small:

- `search` uses Exa and returns normalized JSON.
- `fetch` performs a simple native HTTP fetch and returns Markdown content in JSON.
- JSON is the default output because harnesses are the primary caller.
- `fetch --md` prints only the Markdown body for quick human inspection.

## Usage

```bash
webtools search "rust async runtime comparison"
webtools search --count 10 "openai responses api web search"

webtools fetch https://example.com
webtools fetch --md https://example.com
```

`search` requires `EXA_API_KEY`.

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

## Non-goals

- no browser automation
- no crawler
- no JavaScript rendering
- no enterprise-grade network stack
- no runtime Defuddle/Node dependency

The fetcher should be excellent on common docs, blogs, changelogs, articles, raw files, and JSON endpoints. Weird pages can fail cheaply and clearly.

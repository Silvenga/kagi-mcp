---
name: kagi
description: |
  Use the Kagi search and extract tools to find current web information and read URLs into clean markdown. Fire when you 
  need to look something up, need information outside your training data, need to read a URL, or are 
  launching a web research task - even if "Kagi" isn't named.
license: MIT
---

# Kagi Search & Extract

Web search and page extraction via the Kagi tools. Every call bills the User.

## When to use

- Information newer or more specific than your training data.
- A User asks to "look up", "search", "find out", "research", or "check" something.
- You need a specific URL's content in readable form.
- A research task needs multiple web sources.

## When NOT to use

- The answer is in context or the local codebase - read those first.
- Static, well-known knowledge (language syntax, standard library APIs). Don't bill the User to confirm what you already
  know.
- You need a file the User pointed you at - download it directly instead of extracting.

## Cost model

Every API request bills the User. Search is priced at 3:1, compared to extract.

- Craft one strong query. Read its full response before searching again - another search without digesting the first is
  wasted money. Real thinking is required between search calls: analyze the results you got, identify what's still
  unanswered, then decide whether another search is justified. Multiple searches issued together without reasoning
  between them multiplies cost without multiplying insight.
- Extract batches 1–10 URLs per call. Extraction is much cheaper than search, so an extra extract call is a minor cost -
  but batching URLs you already plan to read into one call saves money. Delay extraction until you've gathered the URLs
  you need when practical, but don't avoid extraction entirely just to batch.
- Caching is on by default; repeat queries are free. Don't disable caching unless the User accepts the cost of
  freshness. The cache TTL is configured by the User, trust the User's setting.

### Search budget

Each task has a search budget. When you hit the limit, you must ask the User before searching further. Classify the task
into one of three tiers at the start:

| Tier     | When to use                                                                        | Budget      |
|----------|------------------------------------------------------------------------------------|-------------|
| Quick    | A single lookup or factual question.                                               | 5 searches  |
| Moderate | Comparing options, investigating one system, or a question with a few sub-angles.  | 12 searches |
| Deep     | Multi-system research, architecture exploration, or a question with many unknowns. | 30 searches |

Re-classify upward only if the task's scope genuinely grows. When you reach the budget, tell the User how many searches
you've used, what you've found, and ask whether to continue.

## Tools

### `search` - find web content

- `query` *(required)* - the search query. Kagi operators go inline here. See the Operators section below.
- `workflow` *(optional)* - `images`, `videos`, `news`, or `podcasts`. Omit for general web.
- `after` / `before` *(optional, `YYYY-MM-DD`)* - date window for time-sensitive queries.
- `output_format` *(optional)* - `markdown` (default) or `json`. Prefer markdown.
- `limit_per_domain` *(optional, ≥ 1)* - cap results per domain. Use when results feel repetitive from one site.
- `cache` *(optional, default `true`)* - keep `true` unless freshness is critical.

### `extract` - read URLs into markdown

Fetches 1–10 URLs in one call, returns clean markdown per page.

- `pages` *(required)* - 1–10 HTTPS URLs.
- `output_format` *(optional)* - `markdown` (default) or `json`. Prefer markdown.
- `cache` *(optional, default `true`)* - keep `true` unless freshness is critical. Errors are not cached, do not disable
  the cache because extraction failed - this only wastes the User's money.

### `usage` - view metrics

Free. API/cache metrics. Use when the User asks about spend, or to diagnose stale cache.

## Search Operators

Placed inline in the `search` tool's `query` string.

### `filetype:`

Restrict to a file extension. [Supported extensions](./references/supported-file-types.md). Not all extensions are
supported.

```
us census 1860 filetype:pdf
```

### `site:`

Restrict to a website (eTLD+1). Negate with `-site:` to exclude.

```
best in show dog site:akc.org
kagi search api -site:kagi.com/pricing
```

### `inurl:`

URL must include a term or phrase. Negate with `-inurl:` to exclude.

```
best headphones inurl:forum
rust async runtime -inurl:reddit
```

### `intitle:`

Title must include a term or phrase. Negate with `-intitle:` to exclude.

```
intitle:"breaking changes" tailwind css
docker tutorial -intitle:guide
```

### `"exact phrase"`

Exact words in order. Negate with `-"..."` to exclude a phrase.

```
"cumulative layout shift" core web vitals
rust async -"tokio" site:reddit.com
```

### `()` - grouping

Group words for `AND` / `OR` logic.

```
sweaters (christmas AND ugly)
recipes (szechuan OR cantonese)
```

### `AND` / `OR`

All terms, or either term. Implicit between bare words; explicit inside groups.

```
error handling (Result OR Either) rust
```

### `+` / `-` - require / exclude

`+` requires a term; `-` excludes it.

```
food +cat -dog
```

### `*` - wildcard

Matches any single word.

```
best * ever
```

## Procedure

1. **Decide if you need to search.** Can context, the codebase, or stable knowledge answer it? If yes, don't.
2. **Craft one strong query.** Use operators to narrow before broadening.
3. **Run the search. Read the full response** before considering another search.
4. **If it didn't answer it, refine the operators** (`site:`, `-exclude`, date window) and search again. A refined query
   beats another unrefined one - but each additional search costs the User, so only continue if the previous response
   shows a refinement would help.
5. **If you need content from specific result URLs, extract them in one call when practical.** Only extract URLs you'll
   actually use - don't extract just because results were returned. Batch URLs into one call when it's easy to do so,
   but don't avoid extraction just to batch - extraction is much cheaper than search.
6. **Read and synthesize.** If a page came back empty, see below before re-attempting.
7. **Run `usage` only when the User asks** about spend.

## Extraction failure modes

`extract` is an HTML-only reader. Empty results typically mean the page isn't extractable, not that it has no content.
Common causes:

- **Non-HTML** (PDFs, images, binaries) -> empty. Download files directly or find an HTML summary.
- **404** -> empty, even if the error page has HTML. Treat empty results on suspect URLs as dead links.
- **JSON API endpoints** (e.g. `api.github.com`) -> empty. Call APIs directly.
- **Anti-bot blocks** (Cloudflare, Akamai) -> empty or an error page. GitHub is notorious for this - all GitHub URLs tend
  to be blocked equally.

### When extract returns empty

1. **Notify the User** which URL failed and the likely cause, then continue. Once you've tried two alternatives and the
   result would materially help, ask the User what to do.
2. **Check the URL type** - PDF, API, 404? Switch approach accordingly.
3. **If blocked**, suggest an alternative source or ask the User to paste the content.

If extract returns a short canned message for a domain, the User configured a fallback - honor it. `extract` shouldn't
be used there.

## Anti-patterns

- **Using `output_format: json` unless asked.** Markdown is smaller and optimized for your consumption.
- **Searching to confirm stable knowledge.** If training data covers it, don't bill the User.
- **Treating an empty extract as "no content".** It typically means blocked, dead, or non-HTML - tell the User.

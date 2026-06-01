## Usage for {{ month_str }}

| Day | Extract Reqs | Search Reqs | Cached Extracts | Cached Searches | Failed Extracts |
|-----|-------------|------------|----------------|----------------|----------------|
{% for row in rows %}| {{ row.day }} | {{ row.extract_reqs }} | {{ row.search_reqs }} | {{ row.cached_extracts }} | {{ row.cached_searches }} | {{ row.failed_extracts }} |
{% endfor %}| **Total** | {{ totals.extract_reqs }} | {{ totals.search_reqs }} | {{ totals.cached_extracts }} | {{ totals.cached_searches }} | {{ totals.failed_extracts }} |
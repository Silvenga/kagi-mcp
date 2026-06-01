CREATE TABLE IF NOT EXISTS metrics (
    year INTEGER NOT NULL, month INTEGER NOT NULL, day INTEGER NOT NULL,
    total_extract_requests INTEGER NOT NULL DEFAULT 0,
    total_search_requests INTEGER NOT NULL DEFAULT 0,
    total_extract_urls_from_cache INTEGER NOT NULL DEFAULT 0,
    total_search_requests_from_cache INTEGER NOT NULL DEFAULT 0,
    failed_extract_urls INTEGER NOT NULL DEFAULT 0,
    PRIMARY KEY (year, month, day)
);
CREATE INDEX IF NOT EXISTS idx_metrics_year_month ON metrics(year, month);

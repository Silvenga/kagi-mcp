# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

- `ExtractData.error` field added to the Kagi API response model. Per-URL extraction errors are now surfaced in tool output when the API provides them.
- Extract results now include explicit error messages for any URLs that could not be extracted, rather than returning blank results.

### Changed

- OpenAPI spec updated to the latest Kagi API specification.

### Removed

- `--split-extract-requests` CLI flag and `KAGI_SPLIT_EXTRACT_REQUESTS` environment variable removed. The extract tool now always uses batch mode (single API call for all URLs). Per-URL errors from the batch response are surfaced individually.

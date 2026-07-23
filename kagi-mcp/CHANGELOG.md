# Changelog

## [0.4.2](https://github.com/Silvenga/kagi-mcp/compare/kagi-mcp-v0.4.1...kagi-mcp-v0.4.2) (2026-07-23)


### Bug Fixes

* **tools:** coerce empty strings to None for optional tool params ([#106](https://github.com/Silvenga/kagi-mcp/issues/106)) ([05716e6](https://github.com/Silvenga/kagi-mcp/commit/05716e6717596a619152fc2b789d547960acb43b))

## [0.4.1](https://github.com/Silvenga/kagi-mcp/compare/kagi-mcp-v0.4.0...kagi-mcp-v0.4.1) (2026-07-09)


### Bug Fixes

* **deps:** bump rmcp from 1.7.0 to 2.1.0 ([#97](https://github.com/Silvenga/kagi-mcp/issues/97)) ([3375f8a](https://github.com/Silvenga/kagi-mcp/commit/3375f8a792fe082e04cde4e16ae53ffa81e213c2))

## [0.4.0](https://github.com/Silvenga/kagi-mcp/compare/kagi-mcp-v0.3.0...kagi-mcp-v0.4.0) (2026-06-20)


### Features

* **usage:** add usage metrics tool with MetricsStore ([#73](https://github.com/Silvenga/kagi-mcp/issues/73)) ([d012116](https://github.com/Silvenga/kagi-mcp/commit/d012116637c13a01a600c76d326418a6ddeacbff))

## [0.3.0](https://github.com/Silvenga/kagi-mcp/compare/kagi-mcp-v0.2.2...kagi-mcp-v0.3.0) (2026-05-31)


### Features

* add panic logging via log-panics ([#58](https://github.com/Silvenga/kagi-mcp/issues/58)) ([63ab05e](https://github.com/Silvenga/kagi-mcp/commit/63ab05efc0126103e90d21baddff90d2b5442b99))
* add tracing-based logging with daily rotation ([#55](https://github.com/Silvenga/kagi-mcp/issues/55)) ([319ab4f](https://github.com/Silvenga/kagi-mcp/commit/319ab4f2ce310a5f826c59da480f51722938e84e))
* **api:** sync with latest Kagi API spec and add per-URL error handling ([#69](https://github.com/Silvenga/kagi-mcp/issues/69)) ([fa10acb](https://github.com/Silvenga/kagi-mcp/commit/fa10acb68faf32f120833a54937e8b35c49527be))
* **extract:** add per-domain fallback messages for extract tool ([#45](https://github.com/Silvenga/kagi-mcp/issues/45)) ([1e33074](https://github.com/Silvenga/kagi-mcp/commit/1e33074c9e637edbae11caeda1678ef6e26db44a))
* use OutputFormat enum instead of raw string ([#62](https://github.com/Silvenga/kagi-mcp/issues/62)) ([74cc0af](https://github.com/Silvenga/kagi-mcp/commit/74cc0afc425e25ea16a91dc3ad847747b4149e37))


### Bug Fixes

* **api:** allow null title in SearchResult to handle Kagi API bug ([#72](https://github.com/Silvenga/kagi-mcp/issues/72)) ([d96a087](https://github.com/Silvenga/kagi-mcp/commit/d96a0877efa9fd860580685047f0c23c2e892072))
* propagate template render error instead of unwrapping ([#63](https://github.com/Silvenga/kagi-mcp/issues/63)) ([49bb1ba](https://github.com/Silvenga/kagi-mcp/commit/49bb1baab9f42634ed784deff06969f637787d2e))
* use generic truncation notice instead of search-specific advice ([#65](https://github.com/Silvenga/kagi-mcp/issues/65)) ([14c5efc](https://github.com/Silvenga/kagi-mcp/commit/14c5efcff0299d3e408d180c8d3ada39351ac132))
* validate after/before date params match YYYY-MM-DD format ([#67](https://github.com/Silvenga/kagi-mcp/issues/67)) ([3adc69a](https://github.com/Silvenga/kagi-mcp/commit/3adc69aefdd97077bd7767675077825cc0838769))

## [0.2.2](https://github.com/Silvenga/kagi-mcp/compare/kagi-mcp-v0.2.1...kagi-mcp-v0.2.2) (2026-05-17)


### Bug Fixes

* avoid nullable type unions in search tool schema for LLM compatibility ([#41](https://github.com/Silvenga/kagi-mcp/issues/41)) ([2d0eaa6](https://github.com/Silvenga/kagi-mcp/commit/2d0eaa6442824556a70e884895b28279b97b18b4))
* **cache:** use platform-specific default cache directory ([#43](https://github.com/Silvenga/kagi-mcp/issues/43)) ([eb8c1d2](https://github.com/Silvenga/kagi-mcp/commit/eb8c1d2b9a227023168d18e03f80077540044a6f))

## [0.2.1](https://github.com/Silvenga/kagi-mcp/compare/kagi-mcp-v0.2.0...kagi-mcp-v0.2.1) (2026-05-17)


### Bug Fixes

* disable allowed hosts validation in streamable HTTP mode ([#39](https://github.com/Silvenga/kagi-mcp/issues/39)) ([89d244d](https://github.com/Silvenga/kagi-mcp/commit/89d244d0d9202317deaf4817685d08abe37f9913))

## [0.2.0](https://github.com/Silvenga/kagi-mcp/compare/kagi-mcp-v0.1.0...kagi-mcp-v0.2.0) (2026-05-17)


### Features

* add API client and server scaffold with validation ([cb40890](https://github.com/Silvenga/kagi-mcp/commit/cb408903b1d04319528acf933a1fca97402407c9))
* add cancellation, progress, and comprehensive test suites ([68e21f8](https://github.com/Silvenga/kagi-mcp/commit/68e21f855cd709353ded0011be8ea33a7ed86334))
* align kagi-api with latest OpenAPI spec and update MCP defaults ([#23](https://github.com/Silvenga/kagi-mcp/issues/23)) ([8a187e5](https://github.com/Silvenga/kagi-mcp/commit/8a187e5b43b8e93a2e468b37695d3555d075d790))
* **cache:** add SQLite-based response caching layer ([#19](https://github.com/Silvenga/kagi-mcp/issues/19)) ([fbfc407](https://github.com/Silvenga/kagi-mcp/commit/fbfc407461adbf9a2cf551067922bb007603cf50))
* **cache:** migrate from rusqlite to sqlx with sqlite and migrations ([#27](https://github.com/Silvenga/kagi-mcp/issues/27)) ([6a3d364](https://github.com/Silvenga/kagi-mcp/commit/6a3d3646a76adc7822b03545224591ed6ffaff16))
* expose search result props and add extract request splitting ([#24](https://github.com/Silvenga/kagi-mcp/issues/24)) ([b357e31](https://github.com/Silvenga/kagi-mcp/commit/b357e315b43c093d213592aa198e93a49232b17e))
* implement search and extract tools with cancellation and progress ([58e236f](https://github.com/Silvenga/kagi-mcp/commit/58e236f8dd8900399d13dc88bf3ddb0a380be97d))
* implement search, extract, markdown formatting, and size guard ([c837f33](https://github.com/Silvenga/kagi-mcp/commit/c837f338332f43609c3d873ddad144220247af31))
* initialize workspace and crate scaffolding ([4d68bf3](https://github.com/Silvenga/kagi-mcp/commit/4d68bf31a5cf2c77556db18b5c7c8a184494eac3))
* **mcp:** add streamable-http transport with Docker and CI support ([#31](https://github.com/Silvenga/kagi-mcp/issues/31)) ([2e37bb9](https://github.com/Silvenga/kagi-mcp/commit/2e37bb9524f59c705534a60fbfe8027c2b9fe7d7))
* polish, clippy cleanup, and documentation ([11fceba](https://github.com/Silvenga/kagi-mcp/commit/11fcebac477447dbd8f9485270099e886d222647))
* **search:** improve markdown output for agent consumption ([#15](https://github.com/Silvenga/kagi-mcp/issues/15)) ([ea792ef](https://github.com/Silvenga/kagi-mcp/commit/ea792efaf3aebff8f66b341399a8a639ce14b0c7))
* split timeouts, migrate to Askama templates, and cleanup ([#17](https://github.com/Silvenga/kagi-mcp/issues/17)) ([9d41f0f](https://github.com/Silvenga/kagi-mcp/commit/9d41f0fb3ad0cf56efb6f78b6fe168389343560b))


### Bug Fixes

* **ci:** resolve release-please workspace compatibility ([#9](https://github.com/Silvenga/kagi-mcp/issues/9)) ([18e5735](https://github.com/Silvenga/kagi-mcp/commit/18e57352b2b0fcdb625ff2c8ebfd3b6c124e98f1))
* **config:** expand tilde in cache dir and lowercase region ([#21](https://github.com/Silvenga/kagi-mcp/issues/21)) ([d3be0d5](https://github.com/Silvenga/kagi-mcp/commit/d3be0d5e30bb31a338998275f1ddad6a123261fe))
* **kagi-mcp:** apply code review fixes — config wiring, error codes, validation, tests ([67cb9a9](https://github.com/Silvenga/kagi-mcp/commit/67cb9a9f249867449d276d35e68764180503d2b3))
* release readiness - various polish and consistency fixes ([#36](https://github.com/Silvenga/kagi-mcp/issues/36)) ([7a69ad0](https://github.com/Silvenga/kagi-mcp/commit/7a69ad029284d57894cf392676d5760c4b79b620))

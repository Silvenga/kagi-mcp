# Changelog

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

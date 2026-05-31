---
name: manual-mcp-server-testing
description: Manually test an MCP server binary end-to-end using the official MCP Inspector CLI against a live API. Covers token handling, release builds, and comprehensive tool verification.
---

End-to-end manual QA of an MCP server binary using the official `@modelcontextprotocol/inspector` CLI tool against a live API.

## Pre-Flight Checklist

Before running any commands, confirm:

- [ ] Binary path is known and file exists: `ls <path/to/binary>`
- [ ] Node.js is available: `node --version`
- [ ] User has provided API token (see Phase 1)
- [ ] You know the environment variable name the server expects (e.g., `KAGI_API_KEY`)

> **STOP:** If any item is unchecked, ask the user. Do not proceed.

## Phase 1: Obtain Credentials

### Step 1: Ask the User for the API Token

**Do NOT guess, hardcode, or look for an existing token.** Tokens are secrets. Ask the user explicitly.

Prompt the user with something like:

> "To run end-to-end tests against the live API, I need an API token. Please paste a temporary token. I will not write it to disk or commit it."

When the user provides the token:

- **Only pass it via the `-e` flag** to the inspector process (environment variable injection).
- **Never write it to a file.** Do not save it to `.env`, config files, or scripts.
- **Never include it in commit messages, logs, or skill artifacts.**
- If you need it in multiple commands, ask the user again rather than persisting it.

## Phase 2: Build the Release Binary

### Step 2: Compile in Release Mode

Run the release build for the workspace (or crate):

```bash
cargo build --workspace --release
```

Confirm the binary exists at the expected path (e.g., `./target/release/<binary-name>`).

## Phase 3: Inspector CLI Basics

### Step 3: Verify the Inspector is Available

The inspector package does not support `--version`. Use `--cli --help` to confirm the CLI binary is reachable:

```bash
npx @modelcontextprotocol/inspector --cli --help
```

If this prints usage information, the inspector is installed and ready. If it prompts to install, allow it. Do not use `--version` — it is not a recognized flag and will hang.

### Step 4: Discover Available Tools

List the tools the server exposes. This confirms the binary starts and the MCP protocol handshake works.

```bash
npx @modelcontextprotocol/inspector --cli --transport stdio \
  -e <ENV_VAR_NAME>=<token> -- <path/to/binary> \
  --method tools/list
```

Replace `<ENV_VAR_NAME>` with whatever the server reads (e.g., `KAGI_API_KEY`), and `<path/to/binary>` with the compiled binary path.

### Common CLI Mistakes

**Rule:** Everything before `--` is for the inspector. Everything after `--` is the server command.

**❌ WRONG — Using `--params` (this is not valid inspector syntax):**
```bash
npx @modelcontextprotocol/inspector --cli --transport stdio \
  -e KAGI_API_KEY=$TOKEN -- ./target/release/kagi-mcp \
  --params '{"name":"search","arguments":{"query":"test"}}'
```

**❌ WRONG — `-e` after `--` (env vars are inspector flags, not server flags):**
```bash
npx @modelcontextprotocol/inspector --cli --transport stdio \
  -- ./target/release/kagi-mcp \
  -e KAGI_API_KEY=$TOKEN \
  --method tools/list
```

**✅ CORRECT:**
```bash
npx @modelcontextprotocol/inspector --cli --transport stdio \
  -e KAGI_API_KEY=$TOKEN -- ./target/release/kagi-mcp \
  --method tools/list
```

## Phase 4: Configure Test Environment

Before running tests, set up an isolated cache and log directory. This ensures a clean state and lets you verify logging afterward.

### Step 5: Create a Temporary Directory

```bash
TEST_DIR="/tmp/manual-test-$(date +%s)"
mkdir -p "$TEST_DIR"
```

Pass this directory to the server via the cache-dir flag or environment variable. Use the same directory for every inspector invocation in this session.

```bash
npx @modelcontextprotocol/inspector --cli --transport stdio \
  -e KAGI_API_KEY=$TOKEN \
  -- ./target/release/kagi-mcp \
  --cache-dir "$TEST_DIR" \
  --method tools/list
```

If the server uses an environment variable for the cache directory instead of a flag, pass it via `-e`:

```bash
npx @modelcontextprotocol/inspector --cli --transport stdio \
  -e KAGI_API_KEY=$TOKEN \
  -e KAGI_CACHE_DIR="$TEST_DIR" \
  -- ./target/release/kagi-mcp \
  --method tools/list
```

### Step 6: Verify Logging After Tests

Once all tests are complete, inspect the contents of `$TEST_DIR`:

1. **Check that log files exist.** Look for files like `kagi-mcp.log` or dated logs (`kagi-mcp.log.YYYY-MM-DD`) in `$TEST_DIR`.
2. **Verify the API token does not appear in logs.** Search the log files for the token value or fragments of it. If found, flag as a security issue.
   ```bash
   grep -r "<token-fragment>" "$TEST_DIR" || echo "Token not found in logs"
   ```
3. **Check for ERROR or WARN entries.** Look for unexpected errors, panics, or warnings.
   ```bash
   grep -E "ERROR|WARN" "$TEST_DIR"/*.log
   ```
4. **Confirm expected log entries.** Verify that successful tool calls produced INFO-level entries (e.g., "search started", "extract completed").

Record any anomalies in the final report.

## Phase 5: Test Each Tool End-to-End

> **STOP:** Before testing, run `tools/list` and record every tool name. You must test **all** of them.

For each tool, determine which tests apply by checking its schema:

### Decision Tree

1. **Does the tool have required parameters?**
   - Test: Omit one required parameter → expect error `-32602` (Invalid params).

2. **Does the tool accept `output_format`?**
   - Test: Call with `output_format=markdown` → expect markdown content.
   - Test: Call with `output_format=json` → expect JSON content.

3. **Does the tool accept URLs or domains?**
   - Test: Pass `192.168.1.1` or `10.0.0.1` → expect rejection (SSRF guard).

4. **Does the tool have a `workflow` parameter?**
   - Test: Each documented workflow value (images, news, videos, podcasts).

5. **Does the tool accept an array parameter?**
   - Test: Empty array → expect error.
   - Test: Single item → expect success.
   - Test: Maximum allowed items → expect success.
   - Test: Exceeding maximum → expect error.

6. **Happy path**
   - Test: All required params with valid values → expect success.

### Syntax for Calling a Tool

```bash
npx @modelcontextprotocol/inspector --cli --transport stdio \
  -e <ENV_VAR_NAME>=<token> -- <path/to/binary> \
  --method tools/call \
  --tool-name <tool-name> \
  --tool-arg <param1>=<value1> \
  --tool-arg <param2>=<value2>
```

**Important:** Use `--tool-arg` for each parameter. For JSON/array values, pass the raw JSON string as the value (e.g., `--tool-arg pages='["https://example.com/"]'`).

### Expected Output Patterns

**Success:**
```json
{
  "content": [
    { "type": "text", "text": "..." }
  ],
  "isError": false
}
```

**Parameter error:**
```json
{
  "error": {
    "code": -32602,
    "message": "..."
  }
}
```

**Internal error (opaque, indicates a bug):**
```json
{
  "error": {
    "code": -32603,
    "message": "Request failed: error decoding response body"
  }
}
```

### Workflow-Specific Tests

Run the workflow-specific tests from the decision tree above. If a workflow returns an unexpected failure, record the exact error in the report. **Do not investigate the root cause.**

## Phase 6: Report

After running all tests, provide a concise QA summary. **Do not attempt to diagnose or fix failures.** Record them and let the user decide if they want you to investigate.

```text
## QA Report: <server-name> v<version>

### Tests Passed: X/Y
- Tool A: basic call, JSON output, invalid date rejection
- Tool B: single URL, multi-URL batch, private IP rejection

### Tests Failed: Z/Y
- Tool A — workflow="images" fails with "error decoding response body"
- Tool B — invalid date "not-a-date" rejected with "limit_per_domain must be >= 1" (wrong error message?)

### Release Readiness: <READY / NOT READY>
```

If any documented feature is broken, mark **NOT READY** and describe the observed behavior.

> **STOP:** Do not proceed to debugging or root cause analysis unless the user explicitly asks for it. They may want to review the failures themselves first.

## Escalation Triggers

Stop and ask the user if:
- The inspector CLI itself crashes (not an MCP error from the server).
- You are tempted to write a bash wrapper script around the inspector.
- You want to switch from `stdio` to `streamable-http` or `sse` to "work around" a CLI issue.
- The same test fails 3 times with identical errors — this indicates a real bug, not a transient issue.

# Setup

Add below snippet in vibe's `config.toml` file. Assuming using `pnpm`.

```toml
[[mcp_servers]]
name = "drawio"
transport = "stdio"
command = "pnpm"
args = ["dlx", "@drawio/mcp"]
```
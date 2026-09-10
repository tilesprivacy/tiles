# Bundled plugins

Plugins that ship with Tiles. They install to `<lib_dir>/plugins/` and are
replaced on upgrade, so treat them as read-only at runtime.

Users cannot uninstall these. `tiles plugin disable <name>` turns one off
instead, which survives upgrades.

Each one follows the [Agent Plugins](https://agent-plugins.org/) spec: a
`plugin.json` manifest, plus any of `skills/`, `mcp.json`, and
`run.tiles/extensions/`. See `docs/` for the full format.

## exa

Web search and page fetch, backed by Exa's hosted MCP server. Needs no API
key: the free tier covers casual use. Set `EXA_API_KEY` to lift the rate
limits and the header picks it up automatically.

Also carries a `web-research` skill, which is a good example of one plugin
holding two resource types: `mcp.json` provides the tools, `skills/` tells the
model how to use them well.

# Plugins in Tiles

A plugin adds a capability to Tiles. Web search, reading your mail, searching your
notes, whatever you need.

The important part is what users see. They install "Exa" and they get web search. They
never learn that inside it is an MCP server, or that some other plugin is a skill, or
that a third one is running actual code. One thing to install, one thing to think about.

Inside, a plugin can hold three different kinds of content. All three arrive in the same
package and install with the same command.

## The package format

Tiles uses Agent Plugins, a small open spec at agent-plugins.org. Because it is open, a
plugin you write for Tiles can work in other tools too.

A plugin is just a folder:

```text
my-plugin/
├── plugin.json
├── skills/
│   └── deploy/SKILL.md
├── mcp.json
└── run.tiles/
    └── extensions/
        └── hello/index.ts
```

Everything except `plugin.json` is optional. A plugin with nothing but an `mcp.json` is
completely normal. That is exactly what Exa is.

## plugin.json

The only required file. The smallest valid one has two fields:

```json
{
  "$schema": "https://agent-plugins.org/schemas/1.0.0/plugin.schema.json",
  "name": "my-plugin"
}
```

Add a `description` too, because that is the text users actually see when they list
their plugins. You can also set `version`, `author`, `homepage`, `repository`,
`license`, and `keywords`.

Three rules worth knowing before you hit them:

The `$schema` value must be exactly the 1.0.0 or 1.1.0 URL. **Use 1.0.0.** The MCP
adapter we bundle only understands 1.0.0, so a 1.1.0 plugin will install fine and then
quietly have no working MCP servers. You do get a warning, but it is easy to miss.

The `name` can only contain lowercase letters, digits, dots and dashes. It cannot start
or end with a dot or dash, and cannot contain `--` or `..`. Sixty four characters max.

Any field we do not recognise gets reported and ignored. It is not an error.

One thing that catches people out: the `name` inside this file is the real name of the
plugin. It is what you type to uninstall it. The folder name and the tarball name do not
matter.

## Skills

A skill is a markdown file that teaches the model how to do something. Each skill is a
folder with a `SKILL.md` inside it.

```text
skills/
└── deploy/
    ├── SKILL.md
    ├── scripts/rollback.sh
    └── references/runbook.md
```

In the REPL, `/skills` lists them, and `@deploy` runs one directly. The model can also
reach for one on its own when it looks relevant.

## MCP servers

MCP is a standard way for a program to hand tools to an AI. If somebody already
published an MCP server for a service you care about, wrapping it in a plugin is most of
the work done for you.

```json
{
  "$schema": "https://agent-plugins.org/schemas/1.0.0/mcp.schema.json",
  "mcpServers": {
    "search": {
      "type": "streamable-http",
      "url": "https://example.com/mcp"
    }
  }
}
```

There are two kinds of server.

**Remote ones** use `streamable-http` and are just a URL. Nothing to bundle, nothing to
install. This is the easy case. The URL has to be `https`, unless it points at
localhost.

**Local ones** use `stdio` and are a program on the user's machine that Tiles starts for
them:

```json
{
  "search": {
    "type": "stdio",
    "command": "./bin/server.py",
    "args": ["--root", "${PLUGIN_DATA}"],
    "cwd": "${PLUGIN_ROOT}"
  }
}
```

The `command` has to be either a bare name like `python3`, or a path starting with `./`
that stays inside your package. **Absolute paths are rejected.** If you bundle the
program itself, remember to make it executable and give it a shebang line.

You get two placeholders to use in `args`, `env` and `cwd`. `${PLUGIN_ROOT}` is your
package folder, which is read only. `${PLUGIN_DATA}` is a writable folder that Tiles
sets aside for your plugin.

For API keys, put `$env:MY_API_KEY` in a header value and the adapter swaps in the
environment variable when it makes the request. Never put a real key in the file itself.
The package ships to everybody who installs it.

## Extensions

This is for real code running inside Pi. The `run.tiles` folder is our namespace, which
is the spec's way of letting each tool claim its own space.

```text
run.tiles/extensions/hello/index.ts
```

It is TypeScript and there is no build step. Pi runs the file directly.

If your extension needs npm packages, they have to travel inside the package, because
there is no npm available when a user installs it. Either run `npm install` in the
plugin folder and ship the `node_modules` folder alongside your code, or bundle
everything into a single file with esbuild or bun first.

One trap: do not open a dialog from `session_start`. Pi 0.84.2 exits immediately if you
do. Open dialogs from a command or an event handler instead.

## Where plugins live

There are two locations.

Bundled plugins sit in the Tiles library folder. These ship with Tiles itself, they are
read only, and an upgrade replaces them.

Installed plugins sit in your data folder. These are the ones you installed yourself.

If the same name exists in both, the installed one wins. That means you can replace a
plugin we ship without having to rename anything.

## The commands

```bash
tiles plugin install ./my-plugin           # a folder, nothing to pack
tiles plugin install ./my-plugin.tar.gz    # or an archive
tiles plugin install https://example.com/my-plugin.tar.gz
tiles plugin list
tiles plugin disable my-plugin
tiles plugin enable my-plugin
tiles plugin uninstall my-plugin
```

You can point install straight at a folder. There is no need to zip anything while you
are working on a plugin. Archives are still there for sharing one.

A URL just has to return a `.tar.gz` or `.zip`. The address itself does not need to look
like one, because Tiles checks the bytes it downloaded rather than the file name. All of
these work:

```bash
tiles plugin install https://github.com/you/my-plugin/archive/refs/heads/main.tar.gz
tiles plugin install https://api.example.com/repos/you/my-plugin/tarball/main
tiles plugin install "https://files.example.com/export?format=zip"
```

If a URL returns something that is not an archive, like an HTML error page, you get told
so instead of a confusing failure later on.

Installed plugins stay as plain files at `<data>/pi/agent/plugins/<name>/`. Nothing is
packed or hidden, so you can read one with `ls` and `cat`, and so can an agent.

Listing shows you what each plugin is for, rather than what is inside it:

```text
exa  Web search and page fetch  [built-in]
```

That `[built-in]` tag means it shipped with Tiles. Those cannot be uninstalled, because
the next upgrade would just put them back. Disable them instead, which actually sticks.
It gets written into your `config.toml`:

```toml
[plugins]
disabled = ["exa"]
```

Disabling drops the whole plugin at once, so its skills, its MCP servers and its
extensions all switch off together.

**Restart Tiles after any of these commands.** All three kinds of content get wired up
when Pi starts, so a session already running will not notice.

## Writing your own

Say you want to add web search using some MCP server you found.

```bash
mkdir -p my-search
cd my-search

cat > plugin.json <<'EOF'
{
  "$schema": "https://agent-plugins.org/schemas/1.0.0/plugin.schema.json",
  "name": "my-search",
  "version": "1.0.0",
  "description": "Search the web"
}
EOF

cat > mcp.json <<'EOF'
{
  "$schema": "https://agent-plugins.org/schemas/1.0.0/mcp.schema.json",
  "mcpServers": {
    "search": {
      "type": "streamable-http",
      "url": "https://mcp.example.com/mcp"
    }
  }
}
EOF

cd ..
tiles plugin install ./my-search
```

Restart Tiles, then check your work. `/skills` shows the skills you can invoke with
`@`, and `@my-search` followed by a question asks it using that plugin.

To share it, pack it with `tar -czf my-search.tar.gz my-search`. A `.zip` works too.

## Shipping a plugin with Tiles

Drop the folder into `plugins/` in the repo and commit it. The build scripts pick it up
with no extra wiring. It gets copied into the release bundle, installed into the library
folder, and shows up tagged as built-in.

For local development, run `./scripts/setup_dev_layout.sh` once. It symlinks `plugins/`
into your dev folder so `cargo run` can see it.

## How it works underneath

Tiles collects every enabled plugin folder from both locations, then wires up the three
kinds of content three different ways when it starts Pi.

An `mcp.json` gets picked up by path. Tiles writes your plugin folder into
`agentPluginPaths` in the adapter's own config, and the adapter reads your file from
there.

Extensions get passed as `-e <path>` arguments on the Pi command line.

Skills get passed as `--skill <path>` arguments, also on the command line.

Pi itself is started with `--no-extensions`, so nothing loads by accident. The only
things that load are the ones Tiles hands over explicitly.

Skills are never copied anywhere. They load straight from your plugin folder, and that
is precisely what makes disabling work properly.

One detail worth knowing if you ever have to debug this. The MCP adapter caches which
tools each server offers, in a file called `mcp-cache.json` next to its config. It only
asks every server for its tool list when that file is missing. Tiles deletes it whenever
the set of plugins changes, so a newly added server does get asked. If you ever see a
server connect but its tools never appear, delete that file and restart.

## Exa, the one that ships with Tiles

Exa gives you web search and page fetch. It is the simplest plugin possible: a
`plugin.json` and an `mcp.json`, and no code at all.

It needs no API key. Exa's free tier covers casual use. If you want higher rate limits,
set `EXA_API_KEY` in your environment and the header picks it up on its own, because the
`mcp.json` we ship contains this:

```json
"headers": { "x-api-key": "$env:EXA_API_KEY" }
```

When that variable is not set, the header goes out empty, which Exa is perfectly happy
with.

### How the model sees plugin tools

Tiles turns on direct tools, which means each MCP tool is registered as a real tool
with its own name and parameters. The model's tool list ends up looking like this:

```text
read  bash  edit  write
exa__search_web_search_exa
exa__search_web_fetch_exa
```

So asking for something that needs a web search just works. The model calls the tool.

Without direct tools the adapter instead offers proxy tools, one generic `mcp` and one
per server. Everything is still reachable, but the model has to know to route through a
proxy and pass the tool name as an argument, which smaller local models do less
reliably.

The tradeoff is prompt size. Every tool from every plugin takes up room, so if you run a
lot of MCP servers you may prefer the proxies. Turn it off in
`<data>/pi/agent/mcp.json`:

```json
{
  "settings": { "directTools": false }
}
```

Tiles only sets this when the setting is absent, so your choice sticks.

## Things that will trip you up

Forgetting to restart after installing something.

Using an absolute path for a `stdio` server's `command`. It has to be bare or start
with `./`.

Using `$schema` 1.1.0. It installs cleanly and then your MCP servers silently do
nothing.

Shipping an extension without its dependencies.

Putting a symlink in your package that points outside the package. Install will refuse
it. Symlinks that stay inside are fine, which is what npm's `node_modules/.bin` folder
needs.

Expecting `tiles plugin uninstall` to work on a built-in plugin. Disable it instead.

# Assistant benchmark

Checks how well Tiles handles everyday personal-assistant tasks: finding things in notes, keeping lists, planning from a calendar, adding up expenses, drafting a reply, and saying no or "I don't know" when it should. It runs through Tiles' own Pi and model server, so it measures what users get.

Each task starts from a fresh copy of `home/`, a made-up user's notes, tasks, calendar, contacts, expenses and inbox. Dates in it are filled in relative to the day of the run. A task is one or more chats, and each chat is a new Pi session. Checks are plain rules on the reply and the files, so the same run always gets the same verdict.

## Run it

Start Tiles' model server, then:

```sh
tiles server start
python3 benchmarks/assistant/run.py --label baseline
```

Options:

- `--repeats 3` runs each task this many times, since replies vary between runs.
- `--tasks find,careful-email` runs only these groups or task ids.
- `--modelfile` picks the Modelfile whose system prompt is used (default: Gemma 4 12B).
- `--pi` and `--pi-agent` point at another Pi or Pi config.

## Results

Each run writes `results/<date>-<label>/`, which git ignores:

- `summary.json`: the setup used and every attempt's verdict, time, tool calls and final reply.
- `logs/`: Pi's full event stream for each attempt.
- `homes/`: the files as each attempt left them.

Compare two labels to see whether a change helped. The `remember` group needs memory across chats, which Tiles doesn't have yet, so read it separately.

---
name: web-research
description: Research a topic on the web across several sources, verify a claim, or dig past search snippets into full pages. Use for questions needing current information, comparisons, or more than one source. Not needed for a single quick lookup.
---

# Web research

Two tools:

- `web_search_exa` searches and returns titles, URLs, publish dates, and highlights.
- `web_fetch_exa` reads a full page as markdown. Takes one or more URLs.

## Search first, then read if needed

Start with a search. The highlights are often enough. Fetch the full page when:

- The highlights are cut off mid-thought or do not answer the question.
- You need a number, a version, a date, or a quote exactly right.
- The question is about how something works, not just what happened.

Batch URLs into one `web_fetch_exa` call rather than fetching one at a time.

## Writing a good query

Write what you would type into a search box, not a sentence.

- Good: `neovim 0.11 release notes`
- Poor: `can you tell me what is in the latest release of neovim`

Include a date only when the question is genuinely time-bound, and take it from
the current date given to you. Never guess a date from memory. A wrong year sends
the search to the wrong year and the results will look plausible.

If the first search comes back thin, change the wording rather than repeating it.
Try the specific term, the product name, or the error message verbatim.

## Check the dates

Every result has a `Published` field. Use it.

- For "latest" or "current" questions, prefer the most recent result and say when
  it is from.
- If the best result is old, say so rather than presenting it as current.
- Results can disagree. Two sources beat one, especially for numbers.

## Answering

- Say what you found, then where it came from. A URL is enough.
- If sources conflict, say that instead of picking one silently.
- If the search found nothing useful, say so. Do not fill the gap from memory:
  that is the failure this tool exists to prevent.

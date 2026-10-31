//! Reads the daemon's server-sent events, for the CLI

use serde_json::Value;

// a block cut off mid chunk waits for the next one
pub(crate) fn take_events(pending: &mut String) -> Vec<(String, Value)> {
    let mut events = vec![];
    while let Some(end) = pending.find("\n\n") {
        let block: String = pending.drain(..end + 2).collect();
        events.extend(sse_event(&block));
    }
    events
}

// keep-alives come back as none
fn sse_event(block: &str) -> Option<(String, Value)> {
    let mut name = None;
    let mut data = None;
    for line in block.lines() {
        if let Some(value) = line.strip_prefix("event:") {
            name = Some(value.trim().to_owned());
        } else if let Some(value) = line.strip_prefix("data:") {
            data = serde_json::from_str(value.trim()).ok();
        }
    }
    Some((name?, data?))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keep_alives_and_broken_events_are_skipped() {
        let mut pending =
            ":\n\nevent: error\ndata: not json\n\nevent: authorize\ndata: {\"url\":\"u\"}\n\n"
                .to_owned();

        let events = take_events(&mut pending);
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].0, "authorize");
    }
}

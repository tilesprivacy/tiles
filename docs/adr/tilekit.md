# Tilekit APIs

This defines how clients can be integrated with Tiles. Examples of clients are GUIs for Tiles, local AI apps which needs a collaboration layer etc.

## Tilekit apis

Tilekit apis are developed to be used by other developers also if they want to integrate tiles or use any features of Tiles. These are different from "Internal apis" which is kind of tailor made for Tiles official webUI for the plumbing purposes, which might come under tilekit later.

- These apis are standard REST apis under scope `/v1/tilekit/`

**NOTE: We use bruno for local api testing and documentation. Open bruno from `/docs/apis` folder**


- There are mainly 3 groups of apis
  - server apis (/v1/tilekit/server) - For communication with the inference server (thus managing the llama server too)
  - Agent apis (/v1/tilekit/agent`) - For communication with the agent harness (Pi)
  - session apis (/v1/tilekit/session) - For dealing with session specific stuff for ex: fetching a session details, resume a session etc.. (These are mainly the features we have in repl at the moment).

### Server apis

- GET /server/start/ - Starts the py inference server in background
- GET /server/stop/ - stops the py inference server
- GET /server/ping - health check
- GET /server/load-model - Loads the model/download the model. (phase 2)

### Agent apis

Agent apis are there, but these will be abstracted by sessions api too. So consider agent apis are primitives.

- GET /agent/start - Starts the Pi agent
- GET /agent/stop - Stops and kills the Pi agent
- GET /agent/state - Returns the current state of Agent
- GET /agent/end-session - Stops the current session gracefully
- POST /agent/prompt - Sends the user prompt
  - ```json
    {
      "message": "create a todo for today"
    }
    ```
  - The response of this API will SSE events as the clients will be streaming the response and the type of event & data will be following Pi's types.

  - The events will follow Pi events as [here](https://github.com/earendil-works/pi/blob/main/packages/coding-agent/docs/rpc.md#events).

### Session APIs

Sessions apis are supposed to be used for regular activities around chatting with the assistant. This also abstract agent related stuff maximum.

- POST /session/new - Creates a new session. Returns a sessionId. This will start a Pi agent if its not already there.
- POST /session/chat - Save a chat in Tiles internal DB.
- GET /session/chats/\<id> - Fetches chats for a session
- GET /session/{session-id} - Fetch details of a session
- GET /session/list - List details about all the sessions
- GET /session/resume?id=\<session\_id> - Resume a session (phase 2)
- GET /session/search - Search something across sessions (phase 2)

### Account APIs

APIs need to manage user's accounts. Both local and ATproto.

- POST /account/create - create a new local account if not there. Need for onboarding
- GET /account/status - Current status of accounts
- POST /account/set-nickname - Update nicknam

#### Current flow for Streaming assistant

- Acquiring the agent.
  - Use `/session/new` which will create an agent or reuse the current one and return the session Id
  - Use `/agent/prompt` - For agent communication


ATproto apis coming soon...

## Tiles apis

These are internal apis so that Tiles official UIs can communicate with Tiles daemon

#TODO: these will be populated as we develop

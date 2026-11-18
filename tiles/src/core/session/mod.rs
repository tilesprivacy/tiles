/// DialogDB backend for chat sessions
///
/// TODO:
/// - [X] In-memory test for write&query session (session.rs)
/// - [ ] Create a Peer state from the App Signer and put it in the Appstate
/// - [ ] Use the peer state to create entries in dialog db too while doin POST /session/chat api
///     - [ ] Implement needed session implementation
///     - [ ] Implement needed message implementation
/// - [ ] Create a /v2/tilekit/session/list, which fetch from dialogdb
/// - [ ] Wire it to the CLI for demo
/// - [ ] Create a /v2/tilekit/session/{sessino_id}/chats, which fetch from dialogdb
pub mod message;
pub mod session;

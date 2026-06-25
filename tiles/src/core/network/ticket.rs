//! Tickets for Networking
use std::{fmt::Display, str::FromStr};

use iroh::EndpointAddr;
use iroh_gossip::TopicId;
use iroh_tickets::Ticket;

#[derive(serde::Serialize, serde::Deserialize, Debug)]
pub struct LinkTicket {
    pub nickname: String,
    pub did: String,
    pub addr: EndpointAddr,
    pub topic_id: TopicId,
}

impl Ticket for LinkTicket {
    const KIND: &'static str = "link";

    fn encode_bytes(&self) -> Vec<u8> {
        postcard::to_stdvec(&self).expect("linkTicket to bytes couldnt be done")
    }

    fn decode_bytes(bytes: &[u8]) -> Result<Self, iroh_tickets::ParseError> {
        postcard::from_bytes(bytes).map_err(Into::into)
    }
}

impl Display for LinkTicket {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut text = data_encoding::BASE32_NOPAD.encode(&self.encode_bytes()[..]);
        text.make_ascii_lowercase();
        write!(f, "{}", text)
    }
}

impl FromStr for LinkTicket {
    type Err = anyhow::Error;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let ticket_bytes = data_encoding::BASE32_NOPAD.decode(s.to_uppercase().as_bytes())?;
        LinkTicket::decode_bytes(&ticket_bytes).map_err(Into::into)
    }
}

impl LinkTicket {
    pub fn new(topic_id: TopicId, addr: EndpointAddr, did: String, nickname: String) -> Self {
        LinkTicket {
            addr,
            topic_id,
            did,
            nickname,
        }
    }
}

#[derive(serde::Serialize, serde::Deserialize, Debug)]
pub struct EndpointUserData {
    pub did: String,
    pub nickname: String,
}

impl EndpointUserData {
    pub fn new(did: &str, nickname: &str) -> Self {
        Self {
            did: did.to_owned(),
            nickname: nickname.to_owned(),
        }
    }

    fn to_bytes(&self) -> Vec<u8> {
        postcard::to_stdvec(&self).expect("EndpointUserData to bytes couldnt be done")
    }
}

impl TryFrom<String> for EndpointUserData {
    type Error = anyhow::Error;
    fn try_from(value: String) -> Result<Self, Self::Error> {
        let data_bytes = data_encoding::BASE32_NOPAD.decode(value.to_uppercase().as_bytes())?;
        postcard::from_bytes(&data_bytes).map_err(Into::into)
    }
}

impl Display for EndpointUserData {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut text = data_encoding::BASE32_NOPAD.encode(&self.to_bytes()[..]);
        text.make_ascii_lowercase();
        write!(f, "{}", text)
    }
}

#[cfg(test)]
mod tests {
    use crate::core::network::ticket::EndpointUserData;

    #[test]
    fn test_basic_to_fro_userdata_conversion() {
        let user_data = EndpointUserData::new("did:key", "machine");
        let usr_data_str = user_data.to_string();
        let usr_data_struct = EndpointUserData::try_from(usr_data_str).unwrap();

        assert_eq!(user_data.did, usr_data_struct.did);
        assert_eq!(user_data.nickname, usr_data_struct.nickname);
    }
}

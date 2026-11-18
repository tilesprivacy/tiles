/// A chat session
use dialog_corpus::Entity;

#[derive(dialog_query::Attribute, Clone, PartialEq)]
#[domain("run.tiles.session")]
pub struct Name(pub String);

#[derive(dialog_query::Attribute, Clone, PartialEq)]
#[domain("run.tiles.session")]
pub struct CreatorId(pub String);

#[derive(dialog_query::Attribute, Clone, PartialEq)]
#[domain("run.tiles.session")]
pub struct CreatedAt(pub u64);

#[derive(dialog_query::Concept, Clone, Debug)]
pub struct Session {
    pub this: Entity,
    pub name: Name,
    pub created_at: CreatedAt,
    pub creator_id: CreatorId,
}

#[cfg(test)]
mod tests {
    use anyhow::Result;
    use dialog_credentials::{Ed25519Signer, SignerCredential};
    use dialog_effects::storage::Location;
    use dialog_peer::{Allowance, OpenCredential, Peer};
    use dialog_query::{Entity, Output, Query, Term};
    use dialog_repository::Repository;
    use dialog_storage::provider::{CredentialStore, Storage, storage::VolatileSpace};
    use dialog_varsig::Principal;

    use crate::utils::get_unix_time_now;

    use super::*;
    #[tokio::test]
    async fn test_writes_quries_session() -> Result<()> {
        let system = SignerCredential::from(Ed25519Signer::generate().await?);

        let storage = Storage::volatile().owned_by(system.did());

        let creds = CredentialStore::<VolatileSpace>::new().owned_by(system.did());

        let credential = OpenCredential::open("session-test").perform(&creds).await?;

        let peer = Peer::new(credential.clone())
            .at(Location::temp("session-test"))
            .space(Repository::from(credential.did()).branch("main"))
            .with(storage)
            .grant(Allowance::storage(&system))
            .build()
            .await?;

        let branch = peer.state();

        let session_id = Entity::new()?;

        // writing the session
        branch
            .transaction()
            .assert(Session {
                this: session_id.clone(),
                name: Name("session_abc".to_string()),
                created_at: CreatedAt(get_unix_time_now()),
                creator_id: CreatorId(String::from("did:xyz")),
            })
            .commit()
            .publish()
            .perform(&peer)
            .await?;

        let _rows = branch
            .query()
            .select(Query::<Session> {
                this: Term::var("this"),
                name: Term::from("session_abc"),
                created_at: Term::var("created_at"),
                creator_id: Term::var("creator_id"),
            })
            .perform(&peer)
            .try_vec()
            .await?;

        branch
            .transaction()
            .assert(Session {
                this: session_id.clone(),
                name: Name("session_efg".to_string()),
                created_at: CreatedAt(get_unix_time_now()),
                creator_id: CreatorId(String::from("did:xyz")),
            })
            .commit()
            .publish()
            .perform(&peer)
            .await?;

        let rows = branch
            .query()
            .select(Query::<Session> {
                this: session_id.clone().into(),
                name: Term::var("name"),
                created_at: Term::var("created_at"),
                creator_id: Term::var("creator_id"),
            })
            .perform(&peer)
            .try_vec()
            .await?;
        println!("{:?}", rows);
        Ok(())
    }
}

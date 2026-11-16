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
mod tests {}

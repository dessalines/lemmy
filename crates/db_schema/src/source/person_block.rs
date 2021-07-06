use crate::{schema::person_block, PersonBlockId, PersonId};
use serde::Serialize;

#[derive(Clone, Queryable, Associations, Identifiable, PartialEq, Debug, Serialize)]
#[table_name = "person_block"]
pub struct PersonBlock {
  pub id: PersonBlockId,
  pub person_id: PersonId,
  pub recipient_id: PersonId,
  pub published: chrono::NaiveDateTime,
}

#[derive(Insertable, AsChangeset)]
#[table_name = "person_block"]
pub struct PersonBlockForm {
  pub person_id: PersonId,
  pub recipient_id: PersonId,
}

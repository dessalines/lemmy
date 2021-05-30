use crate::{schema::private_message, DbUrl, PersonId, PrimaryLanguageTag, PrivateMessageId};
use serde::Serialize;

#[derive(Clone, Queryable, Associations, Identifiable, PartialEq, Debug, Serialize)]
#[table_name = "private_message"]
pub struct PrivateMessage {
  pub id: PrivateMessageId,
  pub creator_id: PersonId,
  pub recipient_id: PersonId,
  pub content: String,
  pub deleted: bool,
  pub read: bool,
  pub published: chrono::NaiveDateTime,
  pub updated: Option<chrono::NaiveDateTime>,
  pub ap_id: DbUrl,
  pub local: bool,
  pub language: PrimaryLanguageTag,
}

#[derive(Insertable, AsChangeset, Default)]
#[table_name = "private_message"]
pub struct PrivateMessageForm {
  pub creator_id: PersonId,
  pub recipient_id: PersonId,
  pub content: String,
  pub deleted: Option<bool>,
  pub read: Option<bool>,
  pub published: Option<chrono::NaiveDateTime>,
  pub updated: Option<chrono::NaiveDateTime>,
  pub ap_id: Option<DbUrl>,
  pub local: Option<bool>,
  pub language: Option<PrimaryLanguageTag>,
}

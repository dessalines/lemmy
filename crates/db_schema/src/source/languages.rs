use crate::{schema::user_languages, LocalUserId, PrimaryLanguageTag};
use serde::Serialize;

#[derive(Clone, Queryable, Identifiable, PartialEq, Debug, Serialize)]
#[table_name = "user_languages"]
pub struct UserLanguages {
  pub id: i32,
  pub local_user_id: LocalUserId,
  pub language: PrimaryLanguageTag,
}

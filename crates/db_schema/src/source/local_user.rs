use crate::{schema::local_user, LocalUserId, PersonId, PrimaryLanguageTag};
use serde::Serialize;

#[derive(Clone, Queryable, Identifiable, PartialEq, Debug, Serialize)]
#[table_name = "local_user"]
pub struct LocalUser {
  pub id: LocalUserId,
  pub person_id: PersonId,
  pub password_encrypted: String,
  pub email: Option<String>,
  pub show_nsfw: bool,
  pub theme: String,
  pub default_sort_type: i16,
  pub default_listing_type: i16,
  pub interface_language: String,
  pub show_avatars: bool,
  pub send_notifications_to_email: bool,
  pub validator_time: chrono::NaiveDateTime,
  pub show_bot_accounts: bool,
  pub show_scores: bool,
  pub discussion_languages: Vec<PrimaryLanguageTag>,
  pub show_read_posts: bool,
}

// TODO redo these, check table defaults
#[derive(Insertable, AsChangeset, Clone, Default)]
#[table_name = "local_user"]
pub struct LocalUserForm {
  pub person_id: PersonId,
  pub password_encrypted: String,
  pub email: Option<Option<String>>,
  pub show_nsfw: Option<bool>,
  pub theme: Option<String>,
  pub default_sort_type: Option<i16>,
  pub default_listing_type: Option<i16>,
  pub interface_language: Option<String>,
  pub show_avatars: Option<bool>,
  pub send_notifications_to_email: Option<bool>,
  pub show_bot_accounts: Option<bool>,
  pub show_scores: Option<bool>,
  pub discussion_languages: Option<Vec<PrimaryLanguageTag>>,
  pub show_read_posts: Option<bool>,
}

/// A local user view that removes password encrypted
#[derive(Clone, Queryable, Identifiable, PartialEq, Debug, Serialize)]
#[table_name = "local_user"]
pub struct LocalUserSettings {
  pub id: LocalUserId,
  pub person_id: PersonId,
  pub email: Option<String>,
  pub show_nsfw: bool,
  pub theme: String,
  pub default_sort_type: i16,
  pub default_listing_type: i16,
  pub interface_language: String,
  pub show_avatars: bool,
  pub send_notifications_to_email: bool,
  pub validator_time: chrono::NaiveDateTime,
  pub show_bot_accounts: bool,
  pub show_scores: bool,
  pub show_read_posts: bool,
}

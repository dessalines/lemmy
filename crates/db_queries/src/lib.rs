#[macro_use]
extern crate diesel;
#[macro_use]
extern crate strum_macros;
#[macro_use]
extern crate lazy_static;
// this is used in tests
#[allow(unused_imports)]
#[macro_use]
extern crate diesel_migrations;

#[cfg(test)]
extern crate serial_test;

use diesel::{result::Error, *};
use lemmy_db_schema::{CommunityId, DbUrl, PersonId};
use lemmy_utils::ApiError;
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::{env, env::VarError};
use url::Url;

pub mod aggregates;
pub mod source;

pub type DbPool = diesel::r2d2::Pool<diesel::r2d2::ConnectionManager<diesel::PgConnection>>;

pub trait Crud<Form, IdType> {
  fn create(conn: &PgConnection, form: &Form) -> Result<Self, Error>
  where
    Self: Sized;
  fn read(conn: &PgConnection, id: IdType) -> Result<Self, Error>
  where
    Self: Sized;
  fn update(conn: &PgConnection, id: IdType, form: &Form) -> Result<Self, Error>
  where
    Self: Sized;
  fn delete(_conn: &PgConnection, _id: IdType) -> Result<usize, Error>
  where
    Self: Sized,
  {
    unimplemented!()
  }
}

pub trait Followable<Form> {
  fn follow(conn: &PgConnection, form: &Form) -> Result<Self, Error>
  where
    Self: Sized;
  fn follow_accepted(
    conn: &PgConnection,
    community_id: CommunityId,
    person_id: PersonId,
  ) -> Result<Self, Error>
  where
    Self: Sized;
  fn unfollow(conn: &PgConnection, form: &Form) -> Result<usize, Error>
  where
    Self: Sized;
  fn has_local_followers(conn: &PgConnection, community_id: CommunityId) -> Result<bool, Error>;
}

pub trait Joinable<Form> {
  fn join(conn: &PgConnection, form: &Form) -> Result<Self, Error>
  where
    Self: Sized;
  fn leave(conn: &PgConnection, form: &Form) -> Result<usize, Error>
  where
    Self: Sized;
}

pub trait Likeable<Form, IdType> {
  fn like(conn: &PgConnection, form: &Form) -> Result<Self, Error>
  where
    Self: Sized;
  fn remove(conn: &PgConnection, person_id: PersonId, item_id: IdType) -> Result<usize, Error>
  where
    Self: Sized;
}

pub trait Bannable<Form> {
  fn ban(conn: &PgConnection, form: &Form) -> Result<Self, Error>
  where
    Self: Sized;
  fn unban(conn: &PgConnection, form: &Form) -> Result<usize, Error>
  where
    Self: Sized;
}

pub trait Blockable<Form> {
  fn block(conn: &PgConnection, form: &Form) -> Result<Self, Error>
  where
    Self: Sized;
  fn unblock(conn: &PgConnection, form: &Form) -> Result<usize, Error>
  where
    Self: Sized;
}

pub trait Saveable<Form> {
  fn save(conn: &PgConnection, form: &Form) -> Result<Self, Error>
  where
    Self: Sized;
  fn unsave(conn: &PgConnection, form: &Form) -> Result<usize, Error>
  where
    Self: Sized;
}

pub trait Readable<Form> {
  fn mark_as_read(conn: &PgConnection, form: &Form) -> Result<Self, Error>
  where
    Self: Sized;
  fn mark_as_unread(conn: &PgConnection, form: &Form) -> Result<usize, Error>
  where
    Self: Sized;
}

pub trait Reportable<Form> {
  fn report(conn: &PgConnection, form: &Form) -> Result<Self, Error>
  where
    Self: Sized;
  fn resolve(conn: &PgConnection, report_id: i32, resolver_id: PersonId) -> Result<usize, Error>
  where
    Self: Sized;
  fn unresolve(conn: &PgConnection, report_id: i32, resolver_id: PersonId) -> Result<usize, Error>
  where
    Self: Sized;
}

pub trait ApubObject<Form> {
  fn read_from_apub_id(conn: &PgConnection, object_id: &DbUrl) -> Result<Self, Error>
  where
    Self: Sized;
  fn upsert(conn: &PgConnection, user_form: &Form) -> Result<Self, Error>
  where
    Self: Sized;
}

pub trait MaybeOptional<T> {
  fn get_optional(self) -> Option<T>;
}

impl<T> MaybeOptional<T> for T {
  fn get_optional(self) -> Option<T> {
    Some(self)
  }
}

impl<T> MaybeOptional<T> for Option<T> {
  fn get_optional(self) -> Option<T> {
    self
  }
}

pub trait ToSafe {
  type SafeColumns;
  fn safe_columns_tuple() -> Self::SafeColumns;
}

pub trait ToSafeSettings {
  type SafeSettingsColumns;
  fn safe_settings_columns_tuple() -> Self::SafeSettingsColumns;
}

pub trait ViewToVec {
  type DbTuple;
  fn from_tuple_to_vec(tuple: Vec<Self::DbTuple>) -> Vec<Self>
  where
    Self: Sized;
}

pub fn get_database_url_from_env() -> Result<String, VarError> {
  env::var("LEMMY_DATABASE_URL")
}

#[derive(EnumString, ToString, Debug, Serialize, Deserialize, Clone, Copy)]
pub enum SortType {
  Active,
  Hot,
  New,
  TopDay,
  TopWeek,
  TopMonth,
  TopYear,
  TopAll,
  MostComments,
  NewComments,
}

#[derive(EnumString, ToString, Debug, Serialize, Deserialize, Clone, Copy)]
pub enum ListingType {
  All,
  Local,
  Subscribed,
  Community,
}

#[derive(EnumString, ToString, Debug, Serialize, Deserialize, Clone, Copy)]
pub enum SearchType {
  All,
  Comments,
  Posts,
  Communities,
  Users,
  Url,
}

pub fn from_opt_str_to_opt_enum<T: std::str::FromStr>(opt: &Option<String>) -> Option<T> {
  opt.as_ref().map(|t| T::from_str(t).ok()).flatten()
}

pub fn fuzzy_search(q: &str) -> String {
  let replaced = q.replace(" ", "%");
  format!("%{}%", replaced)
}

pub fn limit_and_offset(page: Option<i64>, limit: Option<i64>) -> (i64, i64) {
  let page = page.unwrap_or(1);
  let limit = limit.unwrap_or(10);
  let offset = limit * (page - 1);
  (limit, offset)
}

pub fn is_email_regex(test: &str) -> bool {
  EMAIL_REGEX.is_match(test)
}

pub fn diesel_option_overwrite(opt: &Option<String>) -> Option<Option<String>> {
  match opt {
    // An empty string is an erase
    Some(unwrapped) => {
      if !unwrapped.eq("") {
        Some(Some(unwrapped.to_owned()))
      } else {
        Some(None)
      }
    }
    None => None,
  }
}

pub fn diesel_option_overwrite_to_url(
  opt: &Option<String>,
) -> Result<Option<Option<DbUrl>>, ApiError> {
  match opt.as_ref().map(|s| s.as_str()) {
    // An empty string is an erase
    Some("") => Ok(Some(None)),
    Some(str_url) => match Url::parse(str_url) {
      Ok(url) => Ok(Some(Some(url.into()))),
      Err(_) => Err(ApiError::err("invalid_url")),
    },
    None => Ok(None),
  }
}

embed_migrations!();

pub fn establish_unpooled_connection() -> PgConnection {
  let db_url = match get_database_url_from_env() {
    Ok(url) => url,
    Err(e) => panic!(
      "Failed to read database URL from env var LEMMY_DATABASE_URL: {}",
      e
    ),
  };
  let conn =
    PgConnection::establish(&db_url).unwrap_or_else(|_| panic!("Error connecting to {}", db_url));
  embedded_migrations::run(&conn).expect("load migrations");
  conn
}

lazy_static! {
  static ref EMAIL_REGEX: Regex =
    Regex::new(r"^[a-zA-Z0-9.!#$%&’*+/=?^_`{|}~-]+@[a-zA-Z0-9-]+(?:\.[a-zA-Z0-9-]+)*$")
      .expect("compile email regex");
}

pub mod functions {
  use diesel::sql_types::*;

  sql_function! {
    fn hot_rank(score: BigInt, time: Timestamp) -> Integer;
  }
}

#[cfg(test)]
mod tests {
  use super::{fuzzy_search, *};
  use crate::is_email_regex;

  #[test]
  fn test_fuzzy_search() {
    let test = "This is a fuzzy search";
    assert_eq!(fuzzy_search(test), "%This%is%a%fuzzy%search%".to_string());
  }

  #[test]
  fn test_email() {
    assert!(is_email_regex("gush@gmail.com"));
    assert!(!is_email_regex("nada_neutho"));
  }

  #[test]
  fn test_diesel_option_overwrite() {
    assert_eq!(diesel_option_overwrite(&None), None);
    assert_eq!(diesel_option_overwrite(&Some("".to_string())), Some(None));
    assert_eq!(
      diesel_option_overwrite(&Some("test".to_string())),
      Some(Some("test".to_string()))
    );
  }

  #[test]
  fn test_diesel_option_overwrite_to_url() {
    assert!(matches!(diesel_option_overwrite_to_url(&None), Ok(None)));
    assert!(matches!(
      diesel_option_overwrite_to_url(&Some("".to_string())),
      Ok(Some(None))
    ));
    assert!(matches!(
      diesel_option_overwrite_to_url(&Some("invalid_url".to_string())),
      Err(_)
    ));
    let example_url = "https://example.com";
    assert!(matches!(
      diesel_option_overwrite_to_url(&Some(example_url.to_string())),
      Ok(Some(Some(url))) if url == Url::parse(example_url).unwrap().into()
    ));
  }
}

use diesel::{result::Error, *};
use lemmy_db_queries::{ToSafe, ViewToVec};
use lemmy_db_schema::{
  schema::{community, community_block, person},
  source::{
    community::{Community, CommunitySafe},
    person::{Person, PersonSafe},
  },
  PersonId,
};
use serde::Serialize;

#[derive(Debug, Serialize, Clone)]
pub struct CommunityBlockView {
  pub person: PersonSafe,
  pub community: CommunitySafe,
}

type CommunityBlockViewTuple = (PersonSafe, CommunitySafe);

impl CommunityBlockView {
  pub fn for_person(conn: &PgConnection, person_id: PersonId) -> Result<Vec<Self>, Error> {
    let res = community_block::table
      .inner_join(person::table)
      .inner_join(community::table)
      .select((
        Person::safe_columns_tuple(),
        Community::safe_columns_tuple(),
      ))
      .filter(community_block::person_id.eq(person_id))
      .order_by(community_block::published)
      .load::<CommunityBlockViewTuple>(conn)?;

    Ok(Self::from_tuple_to_vec(res))
  }
}

impl ViewToVec for CommunityBlockView {
  type DbTuple = CommunityBlockViewTuple;
  fn from_tuple_to_vec(items: Vec<Self::DbTuple>) -> Vec<Self> {
    items
      .iter()
      .map(|a| Self {
        person: a.0.to_owned(),
        community: a.1.to_owned(),
      })
      .collect::<Vec<Self>>()
  }
}

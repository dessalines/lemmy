use crate::Blockable;
use diesel::{dsl::*, result::Error, *};
use lemmy_db_schema::source::community_block::{CommunityBlock, CommunityBlockForm};

impl Blockable<CommunityBlockForm> for CommunityBlock {
  fn block(conn: &PgConnection, community_block_form: &CommunityBlockForm) -> Result<Self, Error> {
    use lemmy_db_schema::schema::community_block::dsl::*;
    insert_into(community_block)
      .values(community_block_form)
      .on_conflict((person_id, community_id))
      .do_update()
      .set(community_block_form)
      .get_result::<Self>(conn)
  }
  fn unblock(
    conn: &PgConnection,
    community_block_form: &CommunityBlockForm,
  ) -> Result<usize, Error> {
    use lemmy_db_schema::schema::community_block::dsl::*;
    diesel::delete(
      community_block
        .filter(person_id.eq(community_block_form.person_id))
        .filter(community_id.eq(community_block_form.community_id)),
    )
    .execute(conn)
  }
}

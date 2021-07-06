use crate::Blockable;
use diesel::{dsl::*, result::Error, *};
use lemmy_db_schema::{
  source::person_block::{PersonBlock, PersonBlockForm},
  PersonId,
};

pub trait PersonBlock_ {
  fn read(
    conn: &PgConnection,
    person_id: PersonId,
    recipient_id: PersonId,
  ) -> Result<PersonBlock, Error>;
}

impl PersonBlock_ for PersonBlock {
  fn read(
    conn: &PgConnection,
    for_person_id: PersonId,
    for_recipient_id: PersonId,
  ) -> Result<Self, Error> {
    use lemmy_db_schema::schema::person_block::dsl::*;
    person_block
      .filter(person_id.eq(for_person_id))
      .filter(recipient_id.eq(for_recipient_id))
      .first::<Self>(conn)
  }
}

impl Blockable<PersonBlockForm> for PersonBlock {
  fn block(conn: &PgConnection, person_block_form: &PersonBlockForm) -> Result<Self, Error> {
    use lemmy_db_schema::schema::person_block::dsl::*;
    insert_into(person_block)
      .values(person_block_form)
      .on_conflict((person_id, recipient_id))
      .do_update()
      .set(person_block_form)
      .get_result::<Self>(conn)
  }
  fn unblock(conn: &PgConnection, person_block_form: &PersonBlockForm) -> Result<usize, Error> {
    use lemmy_db_schema::schema::person_block::dsl::*;
    diesel::delete(
      person_block
        .filter(person_id.eq(person_block_form.person_id))
        .filter(recipient_id.eq(person_block_form.recipient_id)),
    )
    .execute(conn)
  }
}

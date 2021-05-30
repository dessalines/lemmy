use crate::Crud;
use diesel::{dsl::*, result::Error, *};
use lemmy_db_schema::{source::person_mention::*, PersonId, PersonMentionId};

impl Crud<PersonMentionForm, PersonMentionId> for PersonMention {
  fn read(conn: &PgConnection, person_mention_id: PersonMentionId) -> Result<Self, Error> {
    use lemmy_db_schema::schema::person_mention::dsl::*;
    person_mention.find(person_mention_id).first::<Self>(conn)
  }

  fn create(conn: &PgConnection, person_mention_form: &PersonMentionForm) -> Result<Self, Error> {
    use lemmy_db_schema::schema::person_mention::dsl::*;
    // since the return here isnt utilized, we dont need to do an update
    // but get_result doesnt return the existing row here
    insert_into(person_mention)
      .values(person_mention_form)
      .on_conflict((recipient_id, comment_id))
      .do_update()
      .set(person_mention_form)
      .get_result::<Self>(conn)
  }

  fn update(
    conn: &PgConnection,
    person_mention_id: PersonMentionId,
    person_mention_form: &PersonMentionForm,
  ) -> Result<Self, Error> {
    use lemmy_db_schema::schema::person_mention::dsl::*;
    diesel::update(person_mention.find(person_mention_id))
      .set(person_mention_form)
      .get_result::<Self>(conn)
  }
}

pub trait PersonMention_ {
  fn update_read(
    conn: &PgConnection,
    person_mention_id: PersonMentionId,
    new_read: bool,
  ) -> Result<PersonMention, Error>;
  fn mark_all_as_read(
    conn: &PgConnection,
    for_recipient_id: PersonId,
  ) -> Result<Vec<PersonMention>, Error>;
}

impl PersonMention_ for PersonMention {
  fn update_read(
    conn: &PgConnection,
    person_mention_id: PersonMentionId,
    new_read: bool,
  ) -> Result<PersonMention, Error> {
    use lemmy_db_schema::schema::person_mention::dsl::*;
    diesel::update(person_mention.find(person_mention_id))
      .set(read.eq(new_read))
      .get_result::<Self>(conn)
  }

  fn mark_all_as_read(
    conn: &PgConnection,
    for_recipient_id: PersonId,
  ) -> Result<Vec<PersonMention>, Error> {
    use lemmy_db_schema::schema::person_mention::dsl::*;
    diesel::update(
      person_mention
        .filter(recipient_id.eq(for_recipient_id))
        .filter(read.eq(false)),
    )
    .set(read.eq(true))
    .get_results::<Self>(conn)
  }
}

#[cfg(test)]
mod tests {
  use crate::{establish_unpooled_connection, Crud};
  use lemmy_db_schema::{
    source::{
      comment::*,
      community::{Community, CommunityForm},
      person::*,
      person_mention::*,
      post::*,
    },
    PrimaryLanguageTag,
  };
  use serial_test::serial;

  #[test]
  #[serial]
  fn test_crud() {
    let conn = establish_unpooled_connection();

    let new_person = PersonForm {
      name: "terrylake".into(),
      ..PersonForm::default()
    };

    let inserted_person = Person::create(&conn, &new_person).unwrap();

    let recipient_form = PersonForm {
      name: "terrylakes recipient".into(),
      ..PersonForm::default()
    };

    let inserted_recipient = Person::create(&conn, &recipient_form).unwrap();

    let new_community = CommunityForm {
      name: "test community lake".to_string(),
      title: "nada".to_owned(),
      ..CommunityForm::default()
    };

    let inserted_community = Community::create(&conn, &new_community).unwrap();

    let new_post = PostForm {
      name: "A test post".into(),
      creator_id: inserted_person.id,
      community_id: inserted_community.id,
      language: Some(PrimaryLanguageTag("en".to_string())),
      ..PostForm::default()
    };

    let inserted_post = Post::create(&conn, &new_post).unwrap();

    let comment_form = CommentForm {
      content: "A test comment".into(),
      creator_id: inserted_person.id,
      post_id: inserted_post.id,
      language: Some(PrimaryLanguageTag("en".to_string())),
      ..CommentForm::default()
    };

    let inserted_comment = Comment::create(&conn, &comment_form).unwrap();

    let person_mention_form = PersonMentionForm {
      recipient_id: inserted_recipient.id,
      comment_id: inserted_comment.id,
      read: None,
    };

    let inserted_mention = PersonMention::create(&conn, &person_mention_form).unwrap();

    let expected_mention = PersonMention {
      id: inserted_mention.id,
      recipient_id: inserted_mention.recipient_id,
      comment_id: inserted_mention.comment_id,
      read: false,
      published: inserted_mention.published,
    };

    let read_mention = PersonMention::read(&conn, inserted_mention.id).unwrap();
    let updated_mention =
      PersonMention::update(&conn, inserted_mention.id, &person_mention_form).unwrap();
    Comment::delete(&conn, inserted_comment.id).unwrap();
    Post::delete(&conn, inserted_post.id).unwrap();
    Community::delete(&conn, inserted_community.id).unwrap();
    Person::delete(&conn, inserted_person.id).unwrap();
    Person::delete(&conn, inserted_recipient.id).unwrap();

    assert_eq!(expected_mention, read_mention);
    assert_eq!(expected_mention, inserted_mention);
    assert_eq!(expected_mention, updated_mention);
  }
}

use crate::{ApubObject, Crud, Likeable, Readable, Saveable};
use diesel::{dsl::*, result::Error, *};
use lemmy_db_schema::{
  naive_now,
  source::post::{
    Post,
    PostForm,
    PostLike,
    PostLikeForm,
    PostRead,
    PostReadForm,
    PostSaved,
    PostSavedForm,
  },
  CommunityId,
  DbUrl,
  PersonId,
  PostId,
};

impl Crud<PostForm, PostId> for Post {
  fn read(conn: &PgConnection, post_id: PostId) -> Result<Self, Error> {
    use lemmy_db_schema::schema::post::dsl::*;
    post.find(post_id).first::<Self>(conn)
  }

  fn delete(conn: &PgConnection, post_id: PostId) -> Result<usize, Error> {
    use lemmy_db_schema::schema::post::dsl::*;
    diesel::delete(post.find(post_id)).execute(conn)
  }

  fn create(conn: &PgConnection, new_post: &PostForm) -> Result<Self, Error> {
    use lemmy_db_schema::schema::post::dsl::*;
    insert_into(post).values(new_post).get_result::<Self>(conn)
  }

  fn update(conn: &PgConnection, post_id: PostId, new_post: &PostForm) -> Result<Self, Error> {
    use lemmy_db_schema::schema::post::dsl::*;
    diesel::update(post.find(post_id))
      .set(new_post)
      .get_result::<Self>(conn)
  }
}

pub trait Post_ {
  //fn read(conn: &PgConnection, post_id: i32) -> Result<Post, Error>;
  fn list_for_community(
    conn: &PgConnection,
    the_community_id: CommunityId,
  ) -> Result<Vec<Post>, Error>;
  fn update_ap_id(conn: &PgConnection, post_id: PostId, apub_id: DbUrl) -> Result<Post, Error>;
  fn permadelete_for_creator(
    conn: &PgConnection,
    for_creator_id: PersonId,
  ) -> Result<Vec<Post>, Error>;
  fn update_deleted(conn: &PgConnection, post_id: PostId, new_deleted: bool)
    -> Result<Post, Error>;
  fn update_removed(conn: &PgConnection, post_id: PostId, new_removed: bool)
    -> Result<Post, Error>;
  fn update_removed_for_creator(
    conn: &PgConnection,
    for_creator_id: PersonId,
    for_community_id: Option<CommunityId>,
    new_removed: bool,
  ) -> Result<Vec<Post>, Error>;
  fn update_locked(conn: &PgConnection, post_id: PostId, new_locked: bool) -> Result<Post, Error>;
  fn update_stickied(
    conn: &PgConnection,
    post_id: PostId,
    new_stickied: bool,
  ) -> Result<Post, Error>;
  fn is_post_creator(person_id: PersonId, post_creator_id: PersonId) -> bool;
}

impl Post_ for Post {
  fn list_for_community(
    conn: &PgConnection,
    the_community_id: CommunityId,
  ) -> Result<Vec<Self>, Error> {
    use lemmy_db_schema::schema::post::dsl::*;
    post
      .filter(community_id.eq(the_community_id))
      .then_order_by(published.desc())
      .then_order_by(stickied.desc())
      .limit(20)
      .load::<Self>(conn)
  }

  fn update_ap_id(conn: &PgConnection, post_id: PostId, apub_id: DbUrl) -> Result<Self, Error> {
    use lemmy_db_schema::schema::post::dsl::*;

    diesel::update(post.find(post_id))
      .set(ap_id.eq(apub_id))
      .get_result::<Self>(conn)
  }

  fn permadelete_for_creator(
    conn: &PgConnection,
    for_creator_id: PersonId,
  ) -> Result<Vec<Self>, Error> {
    use lemmy_db_schema::schema::post::dsl::*;

    let perma_deleted = "*Permananently Deleted*";
    let perma_deleted_url = "https://deleted.com";

    diesel::update(post.filter(creator_id.eq(for_creator_id)))
      .set((
        name.eq(perma_deleted),
        url.eq(perma_deleted_url),
        body.eq(perma_deleted),
        deleted.eq(true),
        updated.eq(naive_now()),
      ))
      .get_results::<Self>(conn)
  }

  fn update_deleted(
    conn: &PgConnection,
    post_id: PostId,
    new_deleted: bool,
  ) -> Result<Self, Error> {
    use lemmy_db_schema::schema::post::dsl::*;
    diesel::update(post.find(post_id))
      .set((deleted.eq(new_deleted), updated.eq(naive_now())))
      .get_result::<Self>(conn)
  }

  fn update_removed(
    conn: &PgConnection,
    post_id: PostId,
    new_removed: bool,
  ) -> Result<Self, Error> {
    use lemmy_db_schema::schema::post::dsl::*;
    diesel::update(post.find(post_id))
      .set((removed.eq(new_removed), updated.eq(naive_now())))
      .get_result::<Self>(conn)
  }

  fn update_removed_for_creator(
    conn: &PgConnection,
    for_creator_id: PersonId,
    for_community_id: Option<CommunityId>,
    new_removed: bool,
  ) -> Result<Vec<Self>, Error> {
    use lemmy_db_schema::schema::post::dsl::*;

    let mut update = diesel::update(post).into_boxed();
    update = update.filter(creator_id.eq(for_creator_id));

    if let Some(for_community_id) = for_community_id {
      update = update.filter(community_id.eq(for_community_id));
    }

    update
      .set((removed.eq(new_removed), updated.eq(naive_now())))
      .get_results::<Self>(conn)
  }

  fn update_locked(conn: &PgConnection, post_id: PostId, new_locked: bool) -> Result<Self, Error> {
    use lemmy_db_schema::schema::post::dsl::*;
    diesel::update(post.find(post_id))
      .set(locked.eq(new_locked))
      .get_result::<Self>(conn)
  }

  fn update_stickied(
    conn: &PgConnection,
    post_id: PostId,
    new_stickied: bool,
  ) -> Result<Self, Error> {
    use lemmy_db_schema::schema::post::dsl::*;
    diesel::update(post.find(post_id))
      .set(stickied.eq(new_stickied))
      .get_result::<Self>(conn)
  }

  fn is_post_creator(person_id: PersonId, post_creator_id: PersonId) -> bool {
    person_id == post_creator_id
  }
}

impl ApubObject<PostForm> for Post {
  fn read_from_apub_id(conn: &PgConnection, object_id: &DbUrl) -> Result<Self, Error> {
    use lemmy_db_schema::schema::post::dsl::*;
    post.filter(ap_id.eq(object_id)).first::<Self>(conn)
  }

  fn upsert(conn: &PgConnection, post_form: &PostForm) -> Result<Post, Error> {
    use lemmy_db_schema::schema::post::dsl::*;
    insert_into(post)
      .values(post_form)
      .on_conflict(ap_id)
      .do_update()
      .set(post_form)
      .get_result::<Self>(conn)
  }
}

impl Likeable<PostLikeForm, PostId> for PostLike {
  fn like(conn: &PgConnection, post_like_form: &PostLikeForm) -> Result<Self, Error> {
    use lemmy_db_schema::schema::post_like::dsl::*;
    insert_into(post_like)
      .values(post_like_form)
      .on_conflict((post_id, person_id))
      .do_update()
      .set(post_like_form)
      .get_result::<Self>(conn)
  }
  fn remove(conn: &PgConnection, person_id: PersonId, post_id: PostId) -> Result<usize, Error> {
    use lemmy_db_schema::schema::post_like::dsl;
    diesel::delete(
      dsl::post_like
        .filter(dsl::post_id.eq(post_id))
        .filter(dsl::person_id.eq(person_id)),
    )
    .execute(conn)
  }
}

impl Saveable<PostSavedForm> for PostSaved {
  fn save(conn: &PgConnection, post_saved_form: &PostSavedForm) -> Result<Self, Error> {
    use lemmy_db_schema::schema::post_saved::dsl::*;
    insert_into(post_saved)
      .values(post_saved_form)
      .on_conflict((post_id, person_id))
      .do_update()
      .set(post_saved_form)
      .get_result::<Self>(conn)
  }
  fn unsave(conn: &PgConnection, post_saved_form: &PostSavedForm) -> Result<usize, Error> {
    use lemmy_db_schema::schema::post_saved::dsl::*;
    diesel::delete(
      post_saved
        .filter(post_id.eq(post_saved_form.post_id))
        .filter(person_id.eq(post_saved_form.person_id)),
    )
    .execute(conn)
  }
}

impl Readable<PostReadForm> for PostRead {
  fn mark_as_read(conn: &PgConnection, post_read_form: &PostReadForm) -> Result<Self, Error> {
    use lemmy_db_schema::schema::post_read::dsl::*;
    insert_into(post_read)
      .values(post_read_form)
      .on_conflict((post_id, person_id))
      .do_update()
      .set(post_read_form)
      .get_result::<Self>(conn)
  }

  fn mark_as_unread(conn: &PgConnection, post_read_form: &PostReadForm) -> Result<usize, Error> {
    use lemmy_db_schema::schema::post_read::dsl::*;
    diesel::delete(
      post_read
        .filter(post_id.eq(post_read_form.post_id))
        .filter(person_id.eq(post_read_form.person_id)),
    )
    .execute(conn)
  }
}

#[cfg(test)]
mod tests {
  use crate::{establish_unpooled_connection, source::post::*};
  use lemmy_db_schema::{
    source::{
      community::{Community, CommunityForm},
      person::*,
    },
    PrimaryLanguageTag,
  };
  use serial_test::serial;

  #[test]
  #[serial]
  fn test_crud() {
    let conn = establish_unpooled_connection();

    let new_person = PersonForm {
      name: "jim".into(),
      ..PersonForm::default()
    };

    let inserted_person = Person::create(&conn, &new_person).unwrap();

    let new_community = CommunityForm {
      name: "test community_3".to_string(),
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

    let expected_post = Post {
      id: inserted_post.id,
      name: "A test post".into(),
      url: None,
      body: None,
      creator_id: inserted_person.id,
      community_id: inserted_community.id,
      published: inserted_post.published,
      removed: false,
      locked: false,
      stickied: false,
      nsfw: false,
      deleted: false,
      updated: None,
      embed_title: None,
      embed_description: None,
      embed_html: None,
      thumbnail_url: None,
      ap_id: inserted_post.ap_id.to_owned(),
      local: true,
      language: PrimaryLanguageTag("en".to_string()),
    };

    // Post Like
    let post_like_form = PostLikeForm {
      post_id: inserted_post.id,
      person_id: inserted_person.id,
      score: 1,
    };

    let inserted_post_like = PostLike::like(&conn, &post_like_form).unwrap();

    let expected_post_like = PostLike {
      id: inserted_post_like.id,
      post_id: inserted_post.id,
      person_id: inserted_person.id,
      published: inserted_post_like.published,
      score: 1,
    };

    // Post Save
    let post_saved_form = PostSavedForm {
      post_id: inserted_post.id,
      person_id: inserted_person.id,
    };

    let inserted_post_saved = PostSaved::save(&conn, &post_saved_form).unwrap();

    let expected_post_saved = PostSaved {
      id: inserted_post_saved.id,
      post_id: inserted_post.id,
      person_id: inserted_person.id,
      published: inserted_post_saved.published,
    };

    // Post Read
    let post_read_form = PostReadForm {
      post_id: inserted_post.id,
      person_id: inserted_person.id,
    };

    let inserted_post_read = PostRead::mark_as_read(&conn, &post_read_form).unwrap();

    let expected_post_read = PostRead {
      id: inserted_post_read.id,
      post_id: inserted_post.id,
      person_id: inserted_person.id,
      published: inserted_post_read.published,
    };

    let read_post = Post::read(&conn, inserted_post.id).unwrap();
    let updated_post = Post::update(&conn, inserted_post.id, &new_post).unwrap();
    let like_removed = PostLike::remove(&conn, inserted_person.id, inserted_post.id).unwrap();
    let saved_removed = PostSaved::unsave(&conn, &post_saved_form).unwrap();
    let read_removed = PostRead::mark_as_unread(&conn, &post_read_form).unwrap();
    let num_deleted = Post::delete(&conn, inserted_post.id).unwrap();
    Community::delete(&conn, inserted_community.id).unwrap();
    Person::delete(&conn, inserted_person.id).unwrap();

    assert_eq!(expected_post, read_post);
    assert_eq!(expected_post, inserted_post);
    assert_eq!(expected_post, updated_post);
    assert_eq!(expected_post_like, inserted_post_like);
    assert_eq!(expected_post_saved, inserted_post_saved);
    assert_eq!(expected_post_read, inserted_post_read);
    assert_eq!(1, like_removed);
    assert_eq!(1, saved_removed);
    assert_eq!(1, read_removed);
    assert_eq!(1, num_deleted);
  }
}

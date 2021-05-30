use diesel::{result::Error, *};
use lemmy_db_schema::{schema::comment_aggregates, CommentId};
use serde::Serialize;

#[derive(Queryable, Associations, Identifiable, PartialEq, Debug, Serialize, Clone)]
#[table_name = "comment_aggregates"]
pub struct CommentAggregates {
  pub id: i32,
  pub comment_id: CommentId,
  pub score: i64,
  pub upvotes: i64,
  pub downvotes: i64,
  pub published: chrono::NaiveDateTime,
}

impl CommentAggregates {
  pub fn read(conn: &PgConnection, comment_id: CommentId) -> Result<Self, Error> {
    comment_aggregates::table
      .filter(comment_aggregates::comment_id.eq(comment_id))
      .first::<Self>(conn)
  }
}

#[cfg(test)]
mod tests {
  use crate::{
    aggregates::comment_aggregates::CommentAggregates,
    establish_unpooled_connection,
    Crud,
    Likeable,
  };
  use lemmy_db_schema::{
    source::{
      comment::{Comment, CommentForm, CommentLike, CommentLikeForm},
      community::{Community, CommunityForm},
      person::{Person, PersonForm},
      post::{Post, PostForm},
    },
    PrimaryLanguageTag,
  };
  use serial_test::serial;

  #[test]
  #[serial]
  fn test_crud() {
    let conn = establish_unpooled_connection();

    let new_person = PersonForm {
      name: "thommy_comment_agg".into(),
      ..PersonForm::default()
    };

    let inserted_person = Person::create(&conn, &new_person).unwrap();

    let another_person = PersonForm {
      name: "jerry_comment_agg".into(),
      ..PersonForm::default()
    };

    let another_inserted_person = Person::create(&conn, &another_person).unwrap();

    let new_community = CommunityForm {
      name: "TIL_comment_agg".into(),
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

    let child_comment_form = CommentForm {
      content: "A test comment".into(),
      creator_id: inserted_person.id,
      post_id: inserted_post.id,
      parent_id: Some(inserted_comment.id),
      language: Some(PrimaryLanguageTag("en".to_string())),
      ..CommentForm::default()
    };

    let _inserted_child_comment = Comment::create(&conn, &child_comment_form).unwrap();

    let comment_like = CommentLikeForm {
      comment_id: inserted_comment.id,
      post_id: inserted_post.id,
      person_id: inserted_person.id,
      score: 1,
    };

    CommentLike::like(&conn, &comment_like).unwrap();

    let comment_aggs_before_delete = CommentAggregates::read(&conn, inserted_comment.id).unwrap();

    assert_eq!(1, comment_aggs_before_delete.score);
    assert_eq!(1, comment_aggs_before_delete.upvotes);
    assert_eq!(0, comment_aggs_before_delete.downvotes);

    // Add a post dislike from the other person
    let comment_dislike = CommentLikeForm {
      comment_id: inserted_comment.id,
      post_id: inserted_post.id,
      person_id: another_inserted_person.id,
      score: -1,
    };

    CommentLike::like(&conn, &comment_dislike).unwrap();

    let comment_aggs_after_dislike = CommentAggregates::read(&conn, inserted_comment.id).unwrap();

    assert_eq!(0, comment_aggs_after_dislike.score);
    assert_eq!(1, comment_aggs_after_dislike.upvotes);
    assert_eq!(1, comment_aggs_after_dislike.downvotes);

    // Remove the first comment like
    CommentLike::remove(&conn, inserted_person.id, inserted_comment.id).unwrap();
    let after_like_remove = CommentAggregates::read(&conn, inserted_comment.id).unwrap();
    assert_eq!(-1, after_like_remove.score);
    assert_eq!(0, after_like_remove.upvotes);
    assert_eq!(1, after_like_remove.downvotes);

    // Remove the parent post
    Post::delete(&conn, inserted_post.id).unwrap();

    // Should be none found, since the post was deleted
    let after_delete = CommentAggregates::read(&conn, inserted_comment.id);
    assert!(after_delete.is_err());

    // This should delete all the associated rows, and fire triggers
    Person::delete(&conn, another_inserted_person.id).unwrap();
    let person_num_deleted = Person::delete(&conn, inserted_person.id).unwrap();
    assert_eq!(1, person_num_deleted);

    // Delete the community
    let community_num_deleted = Community::delete(&conn, inserted_community.id).unwrap();
    assert_eq!(1, community_num_deleted);
  }
}

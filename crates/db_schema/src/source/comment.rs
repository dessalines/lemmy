use crate::{
  schema::{comment, comment_alias_1, comment_like, comment_saved},
  source::post::Post,
  CommentId,
  DbUrl,
  PersonId,
  PostId,
  PrimaryLanguageTag,
};
use serde::Serialize;

// WITH RECURSIVE MyTree AS (
//     SELECT * FROM comment WHERE parent_id IS NULL
//     UNION ALL
//     SELECT m.* FROM comment AS m JOIN MyTree AS t ON m.parent_id = t.id
// )
// SELECT * FROM MyTree;

#[derive(Clone, Queryable, Associations, Identifiable, PartialEq, Debug, Serialize)]
#[belongs_to(Post)]
#[table_name = "comment"]
pub struct Comment {
  pub id: CommentId,
  pub creator_id: PersonId,
  pub post_id: PostId,
  pub parent_id: Option<CommentId>,
  pub content: String,
  pub removed: bool,
  pub read: bool, // Whether the recipient has read the comment or not
  pub published: chrono::NaiveDateTime,
  pub updated: Option<chrono::NaiveDateTime>,
  pub deleted: bool,
  pub ap_id: DbUrl,
  pub local: bool,
  pub language: PrimaryLanguageTag,
}

#[derive(Clone, Queryable, Associations, Identifiable, PartialEq, Debug, Serialize)]
#[belongs_to(Post)]
#[table_name = "comment_alias_1"]
pub struct CommentAlias1 {
  pub id: CommentId,
  pub creator_id: PersonId,
  pub post_id: PostId,
  pub parent_id: Option<CommentId>,
  pub content: String,
  pub removed: bool,
  pub read: bool, // Whether the recipient has read the comment or not
  pub published: chrono::NaiveDateTime,
  pub updated: Option<chrono::NaiveDateTime>,
  pub deleted: bool,
  pub ap_id: DbUrl,
  pub local: bool,
}

#[derive(Insertable, AsChangeset, Clone, Default)]
#[table_name = "comment"]
pub struct CommentForm {
  pub creator_id: PersonId,
  pub post_id: PostId,
  pub content: String,
  pub parent_id: Option<CommentId>,
  pub removed: Option<bool>,
  pub read: Option<bool>,
  pub published: Option<chrono::NaiveDateTime>,
  pub updated: Option<chrono::NaiveDateTime>,
  pub deleted: Option<bool>,
  pub ap_id: Option<DbUrl>,
  pub local: Option<bool>,
  pub language: Option<PrimaryLanguageTag>,
}

#[derive(Identifiable, Queryable, Associations, PartialEq, Debug, Clone)]
#[belongs_to(Comment)]
#[table_name = "comment_like"]
pub struct CommentLike {
  pub id: i32,
  pub person_id: PersonId,
  pub comment_id: CommentId,
  pub post_id: PostId, // TODO this is redundant
  pub score: i16,
  pub published: chrono::NaiveDateTime,
}

#[derive(Insertable, AsChangeset, Clone)]
#[table_name = "comment_like"]
pub struct CommentLikeForm {
  pub person_id: PersonId,
  pub comment_id: CommentId,
  pub post_id: PostId, // TODO this is redundant
  pub score: i16,
}

#[derive(Identifiable, Queryable, Associations, PartialEq, Debug)]
#[belongs_to(Comment)]
#[table_name = "comment_saved"]
pub struct CommentSaved {
  pub id: i32,
  pub comment_id: CommentId,
  pub person_id: PersonId,
  pub published: chrono::NaiveDateTime,
}

#[derive(Insertable, AsChangeset)]
#[table_name = "comment_saved"]
pub struct CommentSavedForm {
  pub comment_id: CommentId,
  pub person_id: PersonId,
}

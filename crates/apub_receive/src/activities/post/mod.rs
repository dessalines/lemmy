use lemmy_api_common::{blocking, post::PostResponse};
use lemmy_apub::fetcher::{
  objects::get_or_fetch_and_insert_post,
  person::get_or_fetch_and_upsert_person,
};
use lemmy_db_queries::Likeable;
use lemmy_db_schema::{
  source::post::{PostLike, PostLikeForm},
  PostId,
};
use lemmy_db_views::post_view::PostView;
use lemmy_utils::LemmyError;
use lemmy_websocket::{messages::SendPost, LemmyContext, UserOperation};
use url::Url;

pub mod create;
pub mod delete;
pub mod dislike;
pub mod like;
pub mod remove;
pub mod undo_delete;
pub mod undo_dislike;
pub mod undo_like;
pub mod undo_remove;
pub mod update;

async fn send_websocket_message<OP: ToString + Send + lemmy_websocket::OperationType + 'static>(
  post_id: PostId,
  op: OP,
  context: &LemmyContext,
) -> Result<(), LemmyError> {
  let post_view = blocking(context.pool(), move |conn| {
    PostView::read(conn, post_id, None)
  })
  .await??;

  let res = PostResponse { post_view };

  context.chat_server().do_send(SendPost {
    op,
    post: res,
    websocket_id: None,
  });

  Ok(())
}

async fn like_or_dislike_post(
  score: i16,
  actor: &Url,
  object: &Url,
  context: &LemmyContext,
  request_counter: &mut i32,
) -> Result<(), LemmyError> {
  let actor = get_or_fetch_and_upsert_person(actor, context, request_counter).await?;
  let post = get_or_fetch_and_insert_post(object, context, request_counter).await?;

  let post_id = post.id;
  let like_form = PostLikeForm {
    post_id: post.id,
    person_id: actor.id,
    score,
  };
  let person_id = actor.id;
  blocking(context.pool(), move |conn| {
    PostLike::remove(conn, person_id, post_id)?;
    PostLike::like(conn, &like_form)
  })
  .await??;

  send_websocket_message(post.id, UserOperation::CreatePostLike, context).await
}

async fn undo_like_or_dislike_post(
  actor: &Url,
  object: &Url,
  context: &LemmyContext,
  request_counter: &mut i32,
) -> Result<(), LemmyError> {
  let actor = get_or_fetch_and_upsert_person(actor, context, request_counter).await?;
  let post = get_or_fetch_and_insert_post(object, context, request_counter).await?;

  let post_id = post.id;
  let person_id = actor.id;
  blocking(context.pool(), move |conn| {
    PostLike::remove(conn, person_id, post_id)
  })
  .await??;
  send_websocket_message(post.id, UserOperation::CreatePostLike, context).await
}

use lemmy_api_common::{blocking, comment::CommentResponse, send_local_notifs};
use lemmy_apub::fetcher::{
  objects::get_or_fetch_and_insert_comment,
  person::get_or_fetch_and_upsert_person,
};
use lemmy_db_queries::{Crud, Likeable};
use lemmy_db_schema::{
  source::{
    comment::{Comment, CommentLike, CommentLikeForm},
    post::Post,
  },
  CommentId,
  LocalUserId,
};
use lemmy_db_views::comment_view::CommentView;
use lemmy_utils::{utils::scrape_text_for_mentions, LemmyError};
use lemmy_websocket::{messages::SendComment, LemmyContext, UserOperation};
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

async fn get_notif_recipients(
  actor: &Url,
  comment: &Comment,
  context: &LemmyContext,
  request_counter: &mut i32,
) -> Result<Vec<LocalUserId>, LemmyError> {
  let post_id = comment.post_id;
  let post = blocking(context.pool(), move |conn| Post::read(conn, post_id)).await??;
  let actor = get_or_fetch_and_upsert_person(actor, context, request_counter).await?;

  // Note:
  // Although mentions could be gotten from the post tags (they are included there), or the ccs,
  // Its much easier to scrape them from the comment body, since the API has to do that
  // anyway.
  // TODO: for compatibility with other projects, it would be much better to read this from cc or tags
  let mentions = scrape_text_for_mentions(&comment.content);
  send_local_notifs(mentions, comment.clone(), actor, post, context.pool(), true).await
}

// TODO: in many call sites we are setting an empty vec for recipient_ids, we should get the actual
//       recipient actors from somewhere
async fn send_websocket_message<OP: ToString + Send + lemmy_websocket::OperationType + 'static>(
  comment_id: CommentId,
  recipient_ids: Vec<LocalUserId>,
  op: OP,
  context: &LemmyContext,
) -> Result<(), LemmyError> {
  // Refetch the view
  let comment_view = blocking(context.pool(), move |conn| {
    CommentView::read(conn, comment_id, None)
  })
  .await??;

  let res = CommentResponse {
    comment_view,
    recipient_ids,
    form_id: None,
  };

  context.chat_server().do_send(SendComment {
    op,
    comment: res,
    websocket_id: None,
  });

  Ok(())
}

async fn like_or_dislike_comment(
  score: i16,
  actor: &Url,
  object: &Url,
  context: &LemmyContext,
  request_counter: &mut i32,
) -> Result<(), LemmyError> {
  let actor = get_or_fetch_and_upsert_person(actor, context, request_counter).await?;
  let comment = get_or_fetch_and_insert_comment(object, context, request_counter).await?;

  let comment_id = comment.id;
  let like_form = CommentLikeForm {
    comment_id,
    post_id: comment.post_id,
    person_id: actor.id,
    score,
  };
  let person_id = actor.id;
  blocking(context.pool(), move |conn| {
    CommentLike::remove(conn, person_id, comment_id)?;
    CommentLike::like(conn, &like_form)
  })
  .await??;

  send_websocket_message(
    comment_id,
    vec![],
    UserOperation::CreateCommentLike,
    context,
  )
  .await
}

async fn undo_like_or_dislike_comment(
  actor: &Url,
  object: &Url,
  context: &LemmyContext,
  request_counter: &mut i32,
) -> Result<(), LemmyError> {
  let actor = get_or_fetch_and_upsert_person(actor, context, request_counter).await?;
  let comment = get_or_fetch_and_insert_comment(object, context, request_counter).await?;

  let comment_id = comment.id;
  let person_id = actor.id;
  blocking(context.pool(), move |conn| {
    CommentLike::remove(conn, person_id, comment_id)
  })
  .await??;

  send_websocket_message(
    comment.id,
    vec![],
    UserOperation::CreateCommentLike,
    context,
  )
  .await
}

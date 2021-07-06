use crate::PerformCrud;
use actix_web::web::Data;
use lemmy_api_common::{
  blocking,
  check_community_ban,
  check_person_block,
  comment::*,
  get_local_user_view_from_jwt,
  get_post,
  send_local_notifs,
};
use lemmy_apub::{generate_apub_endpoint, ApubLikeableType, ApubObjectType, EndpointType};
use lemmy_db_queries::{source::comment::Comment_, Crud, Likeable};
use lemmy_db_schema::source::comment::*;
use lemmy_db_views::comment_view::CommentView;
use lemmy_utils::{
  utils::{remove_slurs, scrape_text_for_mentions},
  ApiError,
  ConnectionId,
  LemmyError,
};
use lemmy_websocket::{messages::SendComment, LemmyContext, UserOperationCrud};

#[async_trait::async_trait(?Send)]
impl PerformCrud for CreateComment {
  type Response = CommentResponse;

  async fn perform(
    &self,
    context: &Data<LemmyContext>,
    websocket_id: Option<ConnectionId>,
  ) -> Result<CommentResponse, LemmyError> {
    let data: &CreateComment = self;
    let local_user_view = get_local_user_view_from_jwt(&data.auth, context.pool()).await?;

    let content_slurs_removed = remove_slurs(&data.content.to_owned());

    // Check for a community ban
    let post_id = data.post_id;
    let post = get_post(post_id, context.pool()).await?;

    check_community_ban(local_user_view.person.id, post.community_id, context.pool()).await?;

    check_person_block(local_user_view.person.id, post.creator_id, context.pool()).await?;

    // Check if post is locked, no new comments
    if post.locked {
      return Err(ApiError::err("locked").into());
    }

    // If there's a parent_id, check to make sure that comment is in that post
    if let Some(parent_id) = data.parent_id {
      // Make sure the parent comment exists
      let parent = blocking(context.pool(), move |conn| Comment::read(conn, parent_id))
        .await?
        .map_err(|_| ApiError::err("couldnt_create_comment"))?;

      check_person_block(local_user_view.person.id, parent.creator_id, context.pool()).await?;

      // Strange issue where sometimes the post ID is incorrect
      if parent.post_id != post_id {
        return Err(ApiError::err("couldnt_create_comment").into());
      }
    }

    let comment_form = CommentForm {
      content: content_slurs_removed,
      parent_id: data.parent_id.to_owned(),
      post_id: data.post_id,
      creator_id: local_user_view.person.id,
      ..CommentForm::default()
    };

    // Create the comment
    let comment_form2 = comment_form.clone();
    let inserted_comment = blocking(context.pool(), move |conn| {
      Comment::create(conn, &comment_form2)
    })
    .await?
    .map_err(|_| ApiError::err("couldnt_create_comment"))?;

    // Necessary to update the ap_id
    let inserted_comment_id = inserted_comment.id;
    let updated_comment: Comment =
      blocking(context.pool(), move |conn| -> Result<Comment, LemmyError> {
        let apub_id =
          generate_apub_endpoint(EndpointType::Comment, &inserted_comment_id.to_string())?;
        Ok(Comment::update_ap_id(conn, inserted_comment_id, apub_id)?)
      })
      .await?
      .map_err(|_| ApiError::err("couldnt_create_comment"))?;

    updated_comment
      .send_create(&local_user_view.person, context)
      .await?;

    // Scan the comment for user mentions, add those rows
    let post_id = post.id;
    let mentions = scrape_text_for_mentions(&comment_form.content);
    let recipient_ids = send_local_notifs(
      mentions,
      updated_comment.clone(),
      local_user_view.person.clone(),
      post,
      context.pool(),
      true,
    )
    .await?;

    // You like your own comment by default
    let like_form = CommentLikeForm {
      comment_id: inserted_comment.id,
      post_id,
      person_id: local_user_view.person.id,
      score: 1,
    };

    let like = move |conn: &'_ _| CommentLike::like(conn, &like_form);
    if blocking(context.pool(), like).await?.is_err() {
      return Err(ApiError::err("couldnt_like_comment").into());
    }

    updated_comment
      .send_like(&local_user_view.person, context)
      .await?;

    let person_id = local_user_view.person.id;
    let mut comment_view = blocking(context.pool(), move |conn| {
      CommentView::read(conn, inserted_comment.id, Some(person_id))
    })
    .await??;

    // If its a comment to yourself, mark it as read
    let comment_id = comment_view.comment.id;
    if local_user_view.person.id == comment_view.get_recipient_id() {
      blocking(context.pool(), move |conn| {
        Comment::update_read(conn, comment_id, true)
      })
      .await?
      .map_err(|_| ApiError::err("couldnt_update_comment"))?;
      comment_view.comment.read = true;
    }

    let mut res = CommentResponse {
      comment_view,
      recipient_ids,
      form_id: data.form_id.to_owned(),
    };

    context.chat_server().do_send(SendComment {
      op: UserOperationCrud::CreateComment,
      comment: res.clone(),
      websocket_id,
    });

    res.recipient_ids = Vec::new(); // Necessary to avoid doubles

    Ok(res)
  }
}

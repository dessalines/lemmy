use crate::Perform;
use actix_web::web::Data;
use lemmy_api_common::{
  blocking,
  check_community_ban,
  check_downvotes_enabled,
  check_person_block,
  comment::*,
  get_local_user_view_from_jwt,
};
use lemmy_apub::ApubLikeableType;
use lemmy_db_queries::{source::comment::Comment_, Likeable, Saveable};
use lemmy_db_schema::{source::comment::*, LocalUserId};
use lemmy_db_views::{comment_view::CommentView, local_user_view::LocalUserView};
use lemmy_utils::{ApiError, ConnectionId, LemmyError};
use lemmy_websocket::{messages::SendComment, LemmyContext, UserOperation};

#[async_trait::async_trait(?Send)]
impl Perform for MarkCommentAsRead {
  type Response = CommentResponse;

  async fn perform(
    &self,
    context: &Data<LemmyContext>,
    _websocket_id: Option<ConnectionId>,
  ) -> Result<CommentResponse, LemmyError> {
    let data: &MarkCommentAsRead = self;
    let local_user_view = get_local_user_view_from_jwt(&data.auth, context.pool()).await?;

    let comment_id = data.comment_id;
    let orig_comment = blocking(context.pool(), move |conn| {
      CommentView::read(conn, comment_id, None)
    })
    .await??;

    check_community_ban(
      local_user_view.person.id,
      orig_comment.community.id,
      context.pool(),
    )
    .await?;

    // Verify that only the recipient can mark as read
    if local_user_view.person.id != orig_comment.get_recipient_id() {
      return Err(ApiError::err("no_comment_edit_allowed").into());
    }

    // Do the mark as read
    let read = data.read;
    blocking(context.pool(), move |conn| {
      Comment::update_read(conn, comment_id, read)
    })
    .await?
    .map_err(|_| ApiError::err("couldnt_update_comment"))?;

    // Refetch it
    let comment_id = data.comment_id;
    let person_id = local_user_view.person.id;
    let comment_view = blocking(context.pool(), move |conn| {
      CommentView::read(conn, comment_id, Some(person_id))
    })
    .await??;

    let res = CommentResponse {
      comment_view,
      recipient_ids: Vec::new(),
      form_id: None,
    };

    Ok(res)
  }
}

#[async_trait::async_trait(?Send)]
impl Perform for SaveComment {
  type Response = CommentResponse;

  async fn perform(
    &self,
    context: &Data<LemmyContext>,
    _websocket_id: Option<ConnectionId>,
  ) -> Result<CommentResponse, LemmyError> {
    let data: &SaveComment = self;
    let local_user_view = get_local_user_view_from_jwt(&data.auth, context.pool()).await?;

    let comment_saved_form = CommentSavedForm {
      comment_id: data.comment_id,
      person_id: local_user_view.person.id,
    };

    if data.save {
      let save_comment = move |conn: &'_ _| CommentSaved::save(conn, &comment_saved_form);
      if blocking(context.pool(), save_comment).await?.is_err() {
        return Err(ApiError::err("couldnt_save_comment").into());
      }
    } else {
      let unsave_comment = move |conn: &'_ _| CommentSaved::unsave(conn, &comment_saved_form);
      if blocking(context.pool(), unsave_comment).await?.is_err() {
        return Err(ApiError::err("couldnt_save_comment").into());
      }
    }

    let comment_id = data.comment_id;
    let person_id = local_user_view.person.id;
    let comment_view = blocking(context.pool(), move |conn| {
      CommentView::read(conn, comment_id, Some(person_id))
    })
    .await??;

    Ok(CommentResponse {
      comment_view,
      recipient_ids: Vec::new(),
      form_id: None,
    })
  }
}

#[async_trait::async_trait(?Send)]
impl Perform for CreateCommentLike {
  type Response = CommentResponse;

  async fn perform(
    &self,
    context: &Data<LemmyContext>,
    websocket_id: Option<ConnectionId>,
  ) -> Result<CommentResponse, LemmyError> {
    let data: &CreateCommentLike = self;
    let local_user_view = get_local_user_view_from_jwt(&data.auth, context.pool()).await?;

    let mut recipient_ids = Vec::<LocalUserId>::new();

    // Don't do a downvote if site has downvotes disabled
    check_downvotes_enabled(data.score, context.pool()).await?;

    let comment_id = data.comment_id;
    let orig_comment = blocking(context.pool(), move |conn| {
      CommentView::read(conn, comment_id, None)
    })
    .await??;

    check_community_ban(
      local_user_view.person.id,
      orig_comment.community.id,
      context.pool(),
    )
    .await?;

    check_person_block(
      local_user_view.person.id,
      orig_comment.get_recipient_id(),
      context.pool(),
    )
    .await?;

    // Add parent user to recipients
    let recipient_id = orig_comment.get_recipient_id();
    if let Ok(local_recipient) = blocking(context.pool(), move |conn| {
      LocalUserView::read_person(conn, recipient_id)
    })
    .await?
    {
      recipient_ids.push(local_recipient.local_user.id);
    }

    let like_form = CommentLikeForm {
      comment_id: data.comment_id,
      post_id: orig_comment.post.id,
      person_id: local_user_view.person.id,
      score: data.score,
    };

    // Remove any likes first
    let person_id = local_user_view.person.id;
    blocking(context.pool(), move |conn| {
      CommentLike::remove(conn, person_id, comment_id)
    })
    .await??;

    // Only add the like if the score isnt 0
    let comment = orig_comment.comment;
    let do_add = like_form.score != 0 && (like_form.score == 1 || like_form.score == -1);
    if do_add {
      let like_form2 = like_form.clone();
      let like = move |conn: &'_ _| CommentLike::like(conn, &like_form2);
      if blocking(context.pool(), like).await?.is_err() {
        return Err(ApiError::err("couldnt_like_comment").into());
      }

      if like_form.score == 1 {
        comment.send_like(&local_user_view.person, context).await?;
      } else if like_form.score == -1 {
        comment
          .send_dislike(&local_user_view.person, context)
          .await?;
      }
    } else {
      comment
        .send_undo_like(&local_user_view.person, context)
        .await?;
    }

    // Have to refetch the comment to get the current state
    let comment_id = data.comment_id;
    let person_id = local_user_view.person.id;
    let liked_comment = blocking(context.pool(), move |conn| {
      CommentView::read(conn, comment_id, Some(person_id))
    })
    .await??;

    let res = CommentResponse {
      comment_view: liked_comment,
      recipient_ids,
      form_id: None,
    };

    context.chat_server().do_send(SendComment {
      op: UserOperation::CreateCommentLike,
      comment: res.clone(),
      websocket_id,
    });

    Ok(res)
  }
}

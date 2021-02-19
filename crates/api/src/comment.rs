use crate::{
  check_community_ban,
  check_downvotes_enabled,
  collect_moderated_communities,
  get_post,
  get_user_from_jwt,
  get_user_from_jwt_opt,
  is_mod_or_admin,
  Perform,
};
use actix_web::web::Data;
use lemmy_apub::{generate_apub_endpoint, ApubLikeableType, ApubObjectType, EndpointType};
use lemmy_db_queries::{
  source::comment::Comment_,
  Crud,
  Likeable,
  ListingType,
  Reportable,
  Saveable,
  SortType,
};
use lemmy_db_schema::source::{comment::*, comment_report::*, moderator::*};
use lemmy_db_views::{
  comment_report_view::{CommentReportQueryBuilder, CommentReportView},
  comment_view::{CommentQueryBuilder, CommentView},
};
use lemmy_structs::{blocking, comment::*, send_local_notifs};
use lemmy_utils::{
  utils::{remove_slurs, scrape_text_for_mentions},
  APIError,
  ConnectionId,
  LemmyError,
};
use lemmy_websocket::{
  messages::{SendComment, SendModRoomMessage, SendUserRoomMessage},
  LemmyContext,
  UserOperation,
};
use std::str::FromStr;

#[async_trait::async_trait(?Send)]
impl Perform for CreateComment {
  type Response = CommentResponse;

  async fn perform(
    &self,
    context: &Data<LemmyContext>,
    websocket_id: Option<ConnectionId>,
  ) -> Result<CommentResponse, LemmyError> {
    let data: &CreateComment = &self;
    let user = get_user_from_jwt(&data.auth, context.pool()).await?;

    let content_slurs_removed = remove_slurs(&data.content.to_owned());

    // Check for a community ban
    let post_id = data.post_id;
    let post = get_post(post_id, context.pool()).await?;

    check_community_ban(user.id, post.community_id, context.pool()).await?;

    // Check if post is locked, no new comments
    if post.locked {
      return Err(APIError::err("locked").into());
    }

    // If there's a parent_id, check to make sure that comment is in that post
    if let Some(parent_id) = data.parent_id {
      // Make sure the parent comment exists
      let parent =
        match blocking(context.pool(), move |conn| Comment::read(&conn, parent_id)).await? {
          Ok(comment) => comment,
          Err(_e) => return Err(APIError::err("couldnt_find_parent_comment").into()),
        };
      if parent.post_id != post_id {
        return Err(APIError::err("parent_comment_not_in_post").into());
      }
    }

    let comment_form = CommentForm {
      content: content_slurs_removed,
      parent_id: data.parent_id.to_owned(),
      post_id: data.post_id,
      creator_id: user.id,
      removed: None,
      deleted: None,
      read: None,
      published: None,
      updated: None,
      ap_id: None,
      local: true,
    };

    // Create the comment
    let comment_form2 = comment_form.clone();
    let inserted_comment = match blocking(context.pool(), move |conn| {
      Comment::create(&conn, &comment_form2)
    })
    .await?
    {
      Ok(comment) => comment,
      Err(_e) => return Err(APIError::err("couldnt_create_comment").into()),
    };

    // Necessary to update the ap_id
    let inserted_comment_id = inserted_comment.id;
    let updated_comment: Comment =
      match blocking(context.pool(), move |conn| -> Result<Comment, LemmyError> {
        let apub_id =
          generate_apub_endpoint(EndpointType::Comment, &inserted_comment_id.to_string())?;
        Ok(Comment::update_ap_id(&conn, inserted_comment_id, apub_id)?)
      })
      .await?
      {
        Ok(comment) => comment,
        Err(_e) => return Err(APIError::err("couldnt_create_comment").into()),
      };

    updated_comment.send_create(&user, context).await?;

    // Scan the comment for user mentions, add those rows
    let post_id = post.id;
    let mentions = scrape_text_for_mentions(&comment_form.content);
    let recipient_ids = send_local_notifs(
      mentions,
      updated_comment.clone(),
      &user,
      post,
      context.pool(),
      true,
    )
    .await?;

    // You like your own comment by default
    let like_form = CommentLikeForm {
      comment_id: inserted_comment.id,
      post_id,
      user_id: user.id,
      score: 1,
    };

    let like = move |conn: &'_ _| CommentLike::like(&conn, &like_form);
    if blocking(context.pool(), like).await?.is_err() {
      return Err(APIError::err("couldnt_like_comment").into());
    }

    updated_comment.send_like(&user, context).await?;

    let user_id = user.id;
    let mut comment_view = blocking(context.pool(), move |conn| {
      CommentView::read(&conn, inserted_comment.id, Some(user_id))
    })
    .await??;

    // If its a comment to yourself, mark it as read
    let comment_id = comment_view.comment.id;
    if user.id == comment_view.get_recipient_id() {
      match blocking(context.pool(), move |conn| {
        Comment::update_read(conn, comment_id, true)
      })
      .await?
      {
        Ok(comment) => comment,
        Err(_e) => return Err(APIError::err("couldnt_update_comment").into()),
      };
      comment_view.comment.read = true;
    }

    let mut res = CommentResponse {
      comment_view,
      recipient_ids,
      form_id: data.form_id.to_owned(),
    };

    context.chat_server().do_send(SendComment {
      op: UserOperation::CreateComment,
      comment: res.clone(),
      websocket_id,
    });

    res.recipient_ids = Vec::new(); // Necessary to avoid doubles

    Ok(res)
  }
}

#[async_trait::async_trait(?Send)]
impl Perform for EditComment {
  type Response = CommentResponse;

  async fn perform(
    &self,
    context: &Data<LemmyContext>,
    websocket_id: Option<ConnectionId>,
  ) -> Result<CommentResponse, LemmyError> {
    let data: &EditComment = &self;
    let user = get_user_from_jwt(&data.auth, context.pool()).await?;

    let comment_id = data.comment_id;
    let orig_comment = blocking(context.pool(), move |conn| {
      CommentView::read(&conn, comment_id, None)
    })
    .await??;

    check_community_ban(user.id, orig_comment.community.id, context.pool()).await?;

    // Verify that only the creator can edit
    if user.id != orig_comment.creator.id {
      return Err(APIError::err("no_comment_edit_allowed").into());
    }

    // Do the update
    let content_slurs_removed = remove_slurs(&data.content.to_owned());
    let comment_id = data.comment_id;
    let updated_comment = match blocking(context.pool(), move |conn| {
      Comment::update_content(conn, comment_id, &content_slurs_removed)
    })
    .await?
    {
      Ok(comment) => comment,
      Err(_e) => return Err(APIError::err("couldnt_update_comment").into()),
    };

    // Send the apub update
    updated_comment.send_update(&user, context).await?;

    // Do the mentions / recipients
    let updated_comment_content = updated_comment.content.to_owned();
    let mentions = scrape_text_for_mentions(&updated_comment_content);
    let recipient_ids = send_local_notifs(
      mentions,
      updated_comment,
      &user,
      orig_comment.post,
      context.pool(),
      false,
    )
    .await?;

    let comment_id = data.comment_id;
    let user_id = user.id;
    let comment_view = blocking(context.pool(), move |conn| {
      CommentView::read(conn, comment_id, Some(user_id))
    })
    .await??;

    let res = CommentResponse {
      comment_view,
      recipient_ids,
      form_id: data.form_id.to_owned(),
    };

    context.chat_server().do_send(SendComment {
      op: UserOperation::EditComment,
      comment: res.clone(),
      websocket_id,
    });

    Ok(res)
  }
}

#[async_trait::async_trait(?Send)]
impl Perform for DeleteComment {
  type Response = CommentResponse;

  async fn perform(
    &self,
    context: &Data<LemmyContext>,
    websocket_id: Option<ConnectionId>,
  ) -> Result<CommentResponse, LemmyError> {
    let data: &DeleteComment = &self;
    let user = get_user_from_jwt(&data.auth, context.pool()).await?;

    let comment_id = data.comment_id;
    let orig_comment = blocking(context.pool(), move |conn| {
      CommentView::read(&conn, comment_id, None)
    })
    .await??;

    check_community_ban(user.id, orig_comment.community.id, context.pool()).await?;

    // Verify that only the creator can delete
    if user.id != orig_comment.creator.id {
      return Err(APIError::err("no_comment_edit_allowed").into());
    }

    // Do the delete
    let deleted = data.deleted;
    let updated_comment = match blocking(context.pool(), move |conn| {
      Comment::update_deleted(conn, comment_id, deleted)
    })
    .await?
    {
      Ok(comment) => comment,
      Err(_e) => return Err(APIError::err("couldnt_update_comment").into()),
    };

    // Send the apub message
    if deleted {
      updated_comment.send_delete(&user, context).await?;
    } else {
      updated_comment.send_undo_delete(&user, context).await?;
    }

    // Refetch it
    let comment_id = data.comment_id;
    let user_id = user.id;
    let comment_view = blocking(context.pool(), move |conn| {
      CommentView::read(conn, comment_id, Some(user_id))
    })
    .await??;

    // Build the recipients
    let comment_view_2 = comment_view.clone();
    let mentions = vec![];
    let recipient_ids = send_local_notifs(
      mentions,
      updated_comment,
      &user,
      comment_view_2.post,
      context.pool(),
      false,
    )
    .await?;

    let res = CommentResponse {
      comment_view,
      recipient_ids,
      form_id: None, // TODO a comment delete might clear forms?
    };

    context.chat_server().do_send(SendComment {
      op: UserOperation::DeleteComment,
      comment: res.clone(),
      websocket_id,
    });

    Ok(res)
  }
}

#[async_trait::async_trait(?Send)]
impl Perform for RemoveComment {
  type Response = CommentResponse;

  async fn perform(
    &self,
    context: &Data<LemmyContext>,
    websocket_id: Option<ConnectionId>,
  ) -> Result<CommentResponse, LemmyError> {
    let data: &RemoveComment = &self;
    let user = get_user_from_jwt(&data.auth, context.pool()).await?;

    let comment_id = data.comment_id;
    let orig_comment = blocking(context.pool(), move |conn| {
      CommentView::read(&conn, comment_id, None)
    })
    .await??;

    check_community_ban(user.id, orig_comment.community.id, context.pool()).await?;

    // Verify that only a mod or admin can remove
    is_mod_or_admin(context.pool(), user.id, orig_comment.community.id).await?;

    // Do the remove
    let removed = data.removed;
    let updated_comment = match blocking(context.pool(), move |conn| {
      Comment::update_removed(conn, comment_id, removed)
    })
    .await?
    {
      Ok(comment) => comment,
      Err(_e) => return Err(APIError::err("couldnt_update_comment").into()),
    };

    // Mod tables
    let form = ModRemoveCommentForm {
      mod_user_id: user.id,
      comment_id: data.comment_id,
      removed: Some(removed),
      reason: data.reason.to_owned(),
    };
    blocking(context.pool(), move |conn| {
      ModRemoveComment::create(conn, &form)
    })
    .await??;

    // Send the apub message
    if removed {
      updated_comment.send_remove(&user, context).await?;
    } else {
      updated_comment.send_undo_remove(&user, context).await?;
    }

    // Refetch it
    let comment_id = data.comment_id;
    let user_id = user.id;
    let comment_view = blocking(context.pool(), move |conn| {
      CommentView::read(conn, comment_id, Some(user_id))
    })
    .await??;

    // Build the recipients
    let comment_view_2 = comment_view.clone();

    let mentions = vec![];
    let recipient_ids = send_local_notifs(
      mentions,
      updated_comment,
      &user,
      comment_view_2.post,
      context.pool(),
      false,
    )
    .await?;

    let res = CommentResponse {
      comment_view,
      recipient_ids,
      form_id: None, // TODO maybe this might clear other forms
    };

    context.chat_server().do_send(SendComment {
      op: UserOperation::RemoveComment,
      comment: res.clone(),
      websocket_id,
    });

    Ok(res)
  }
}

#[async_trait::async_trait(?Send)]
impl Perform for MarkCommentAsRead {
  type Response = CommentResponse;

  async fn perform(
    &self,
    context: &Data<LemmyContext>,
    _websocket_id: Option<ConnectionId>,
  ) -> Result<CommentResponse, LemmyError> {
    let data: &MarkCommentAsRead = &self;
    let user = get_user_from_jwt(&data.auth, context.pool()).await?;

    let comment_id = data.comment_id;
    let orig_comment = blocking(context.pool(), move |conn| {
      CommentView::read(&conn, comment_id, None)
    })
    .await??;

    check_community_ban(user.id, orig_comment.community.id, context.pool()).await?;

    // Verify that only the recipient can mark as read
    if user.id != orig_comment.get_recipient_id() {
      return Err(APIError::err("no_comment_edit_allowed").into());
    }

    // Do the mark as read
    let read = data.read;
    match blocking(context.pool(), move |conn| {
      Comment::update_read(conn, comment_id, read)
    })
    .await?
    {
      Ok(comment) => comment,
      Err(_e) => return Err(APIError::err("couldnt_update_comment").into()),
    };

    // Refetch it
    let comment_id = data.comment_id;
    let user_id = user.id;
    let comment_view = blocking(context.pool(), move |conn| {
      CommentView::read(conn, comment_id, Some(user_id))
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
    let data: &SaveComment = &self;
    let user = get_user_from_jwt(&data.auth, context.pool()).await?;

    let comment_saved_form = CommentSavedForm {
      comment_id: data.comment_id,
      user_id: user.id,
    };

    if data.save {
      let save_comment = move |conn: &'_ _| CommentSaved::save(conn, &comment_saved_form);
      if blocking(context.pool(), save_comment).await?.is_err() {
        return Err(APIError::err("couldnt_save_comment").into());
      }
    } else {
      let unsave_comment = move |conn: &'_ _| CommentSaved::unsave(conn, &comment_saved_form);
      if blocking(context.pool(), unsave_comment).await?.is_err() {
        return Err(APIError::err("couldnt_save_comment").into());
      }
    }

    let comment_id = data.comment_id;
    let user_id = user.id;
    let comment_view = blocking(context.pool(), move |conn| {
      CommentView::read(conn, comment_id, Some(user_id))
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
    let data: &CreateCommentLike = &self;
    let user = get_user_from_jwt(&data.auth, context.pool()).await?;

    let mut recipient_ids = Vec::new();

    // Don't do a downvote if site has downvotes disabled
    check_downvotes_enabled(data.score, context.pool()).await?;

    let comment_id = data.comment_id;
    let orig_comment = blocking(context.pool(), move |conn| {
      CommentView::read(&conn, comment_id, None)
    })
    .await??;

    check_community_ban(user.id, orig_comment.community.id, context.pool()).await?;

    // Add parent user to recipients
    recipient_ids.push(orig_comment.get_recipient_id());

    let like_form = CommentLikeForm {
      comment_id: data.comment_id,
      post_id: orig_comment.post.id,
      user_id: user.id,
      score: data.score,
    };

    // Remove any likes first
    let user_id = user.id;
    blocking(context.pool(), move |conn| {
      CommentLike::remove(conn, user_id, comment_id)
    })
    .await??;

    // Only add the like if the score isnt 0
    let comment = orig_comment.comment;
    let do_add = like_form.score != 0 && (like_form.score == 1 || like_form.score == -1);
    if do_add {
      let like_form2 = like_form.clone();
      let like = move |conn: &'_ _| CommentLike::like(conn, &like_form2);
      if blocking(context.pool(), like).await?.is_err() {
        return Err(APIError::err("couldnt_like_comment").into());
      }

      if like_form.score == 1 {
        comment.send_like(&user, context).await?;
      } else if like_form.score == -1 {
        comment.send_dislike(&user, context).await?;
      }
    } else {
      comment.send_undo_like(&user, context).await?;
    }

    // Have to refetch the comment to get the current state
    let comment_id = data.comment_id;
    let user_id = user.id;
    let liked_comment = blocking(context.pool(), move |conn| {
      CommentView::read(conn, comment_id, Some(user_id))
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

#[async_trait::async_trait(?Send)]
impl Perform for GetComments {
  type Response = GetCommentsResponse;

  async fn perform(
    &self,
    context: &Data<LemmyContext>,
    _websocket_id: Option<ConnectionId>,
  ) -> Result<GetCommentsResponse, LemmyError> {
    let data: &GetComments = &self;
    let user = get_user_from_jwt_opt(&data.auth, context.pool()).await?;
    let user_id = user.map(|u| u.id);

    let type_ = ListingType::from_str(&data.type_)?;
    let sort = SortType::from_str(&data.sort)?;

    let community_id = data.community_id;
    let community_name = data.community_name.to_owned();
    let page = data.page;
    let limit = data.limit;
    let comments = blocking(context.pool(), move |conn| {
      CommentQueryBuilder::create(conn)
        .listing_type(type_)
        .sort(&sort)
        .community_id(community_id)
        .community_name(community_name)
        .my_user_id(user_id)
        .page(page)
        .limit(limit)
        .list()
    })
    .await?;
    let comments = match comments {
      Ok(comments) => comments,
      Err(_) => return Err(APIError::err("couldnt_get_comments").into()),
    };

    Ok(GetCommentsResponse { comments })
  }
}

/// Creates a comment report and notifies the moderators of the community
#[async_trait::async_trait(?Send)]
impl Perform for CreateCommentReport {
  type Response = CreateCommentReportResponse;

  async fn perform(
    &self,
    context: &Data<LemmyContext>,
    websocket_id: Option<ConnectionId>,
  ) -> Result<CreateCommentReportResponse, LemmyError> {
    let data: &CreateCommentReport = &self;
    let user = get_user_from_jwt(&data.auth, context.pool()).await?;

    // check size of report and check for whitespace
    let reason = data.reason.trim();
    if reason.is_empty() {
      return Err(APIError::err("report_reason_required").into());
    }
    if reason.chars().count() > 1000 {
      return Err(APIError::err("report_too_long").into());
    }

    let user_id = user.id;
    let comment_id = data.comment_id;
    let comment_view = blocking(context.pool(), move |conn| {
      CommentView::read(&conn, comment_id, None)
    })
    .await??;

    check_community_ban(user_id, comment_view.community.id, context.pool()).await?;

    let report_form = CommentReportForm {
      creator_id: user_id,
      comment_id,
      original_comment_text: comment_view.comment.content,
      reason: data.reason.to_owned(),
    };

    let report = match blocking(context.pool(), move |conn| {
      CommentReport::report(conn, &report_form)
    })
    .await?
    {
      Ok(report) => report,
      Err(_e) => return Err(APIError::err("couldnt_create_report").into()),
    };

    let res = CreateCommentReportResponse { success: true };

    context.chat_server().do_send(SendUserRoomMessage {
      op: UserOperation::CreateCommentReport,
      response: res.clone(),
      recipient_id: user.id,
      websocket_id,
    });

    context.chat_server().do_send(SendModRoomMessage {
      op: UserOperation::CreateCommentReport,
      response: report,
      community_id: comment_view.community.id,
      websocket_id,
    });

    Ok(res)
  }
}

/// Resolves or unresolves a comment report and notifies the moderators of the community
#[async_trait::async_trait(?Send)]
impl Perform for ResolveCommentReport {
  type Response = ResolveCommentReportResponse;

  async fn perform(
    &self,
    context: &Data<LemmyContext>,
    websocket_id: Option<ConnectionId>,
  ) -> Result<ResolveCommentReportResponse, LemmyError> {
    let data: &ResolveCommentReport = &self;
    let user = get_user_from_jwt(&data.auth, context.pool()).await?;

    let report_id = data.report_id;
    let report = blocking(context.pool(), move |conn| {
      CommentReportView::read(&conn, report_id)
    })
    .await??;

    let user_id = user.id;
    is_mod_or_admin(context.pool(), user_id, report.community.id).await?;

    let resolved = data.resolved;
    let resolve_fun = move |conn: &'_ _| {
      if resolved {
        CommentReport::resolve(conn, report_id, user_id)
      } else {
        CommentReport::unresolve(conn, report_id, user_id)
      }
    };

    if blocking(context.pool(), resolve_fun).await?.is_err() {
      return Err(APIError::err("couldnt_resolve_report").into());
    };

    let report_id = data.report_id;
    let res = ResolveCommentReportResponse {
      report_id,
      resolved,
    };

    context.chat_server().do_send(SendModRoomMessage {
      op: UserOperation::ResolveCommentReport,
      response: res.clone(),
      community_id: report.community.id,
      websocket_id,
    });

    Ok(res)
  }
}

/// Lists comment reports for a community if an id is supplied
/// or returns all comment reports for communities a user moderates
#[async_trait::async_trait(?Send)]
impl Perform for ListCommentReports {
  type Response = ListCommentReportsResponse;

  async fn perform(
    &self,
    context: &Data<LemmyContext>,
    websocket_id: Option<ConnectionId>,
  ) -> Result<ListCommentReportsResponse, LemmyError> {
    let data: &ListCommentReports = &self;
    let user = get_user_from_jwt(&data.auth, context.pool()).await?;

    let user_id = user.id;
    let community_id = data.community;
    let community_ids =
      collect_moderated_communities(user_id, community_id, context.pool()).await?;

    let page = data.page;
    let limit = data.limit;
    let comments = blocking(context.pool(), move |conn| {
      CommentReportQueryBuilder::create(conn)
        .community_ids(community_ids)
        .page(page)
        .limit(limit)
        .list()
    })
    .await??;

    let res = ListCommentReportsResponse { comments };

    context.chat_server().do_send(SendUserRoomMessage {
      op: UserOperation::ListCommentReports,
      response: res.clone(),
      recipient_id: user.id,
      websocket_id,
    });

    Ok(res)
  }
}

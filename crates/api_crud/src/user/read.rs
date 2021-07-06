use crate::PerformCrud;
use actix_web::web::Data;
use lemmy_api_common::{blocking, get_local_user_view_from_jwt_opt, person::*};
use lemmy_db_queries::{from_opt_str_to_opt_enum, source::person::Person_, SortType};
use lemmy_db_schema::source::person::*;
use lemmy_db_views::{comment_view::CommentQueryBuilder, post_view::PostQueryBuilder};
use lemmy_db_views_actor::{
  community_block_view::CommunityBlockView,
  community_follower_view::CommunityFollowerView,
  community_moderator_view::CommunityModeratorView,
  person_block_view::PersonBlockView,
  person_view::PersonViewSafe,
};
use lemmy_utils::{ApiError, ConnectionId, LemmyError};
use lemmy_websocket::LemmyContext;

#[async_trait::async_trait(?Send)]
impl PerformCrud for GetPersonDetails {
  type Response = GetPersonDetailsResponse;

  async fn perform(
    &self,
    context: &Data<LemmyContext>,
    _websocket_id: Option<ConnectionId>,
  ) -> Result<GetPersonDetailsResponse, LemmyError> {
    let data: &GetPersonDetails = self;
    let local_user_view = get_local_user_view_from_jwt_opt(&data.auth, context.pool()).await?;

    let show_nsfw = local_user_view.as_ref().map(|t| t.local_user.show_nsfw);
    let show_bot_accounts = local_user_view
      .as_ref()
      .map(|t| t.local_user.show_bot_accounts);
    let show_read_posts = local_user_view
      .as_ref()
      .map(|t| t.local_user.show_read_posts);

    let sort: Option<SortType> = from_opt_str_to_opt_enum(&data.sort);

    let username = data
      .username
      .to_owned()
      .unwrap_or_else(|| "admin".to_string());
    let person_details_id = match data.person_id {
      Some(id) => id,
      None => {
        let person = blocking(context.pool(), move |conn| {
          Person::find_by_name(conn, &username)
        })
        .await?;
        person
          .map_err(|_| ApiError::err("couldnt_find_that_username_or_email"))?
          .id
      }
    };

    let person_id = local_user_view.map(|uv| uv.person.id);

    // You don't need to return settings for the user, since this comes back with GetSite
    // `my_user`
    let person_view = blocking(context.pool(), move |conn| {
      PersonViewSafe::read(conn, person_details_id)
    })
    .await??;

    let page = data.page;
    let limit = data.limit;
    let saved_only = data.saved_only;
    let community_id = data.community_id;

    let (posts, comments) = blocking(context.pool(), move |conn| {
      let mut posts_query = PostQueryBuilder::create(conn)
        .sort(sort)
        .show_nsfw(show_nsfw)
        .show_bot_accounts(show_bot_accounts)
        .show_read_posts(show_read_posts)
        .saved_only(saved_only)
        .community_id(community_id)
        .my_person_id(person_id)
        .page(page)
        .limit(limit);

      let mut comments_query = CommentQueryBuilder::create(conn)
        .my_person_id(person_id)
        .show_bot_accounts(show_bot_accounts)
        .sort(sort)
        .saved_only(saved_only)
        .community_id(community_id)
        .page(page)
        .limit(limit);

      // If its saved only, you don't care what creator it was
      // Or, if its not saved, then you only want it for that specific creator
      if !saved_only.unwrap_or(false) {
        posts_query = posts_query.creator_id(person_details_id);
        comments_query = comments_query.creator_id(person_details_id);
      }

      let posts = posts_query.list()?;
      let comments = comments_query.list()?;

      Ok((posts, comments)) as Result<_, LemmyError>
    })
    .await??;

    let mut follows = vec![];
    let mut person_blocks = vec![];
    let mut community_blocks = vec![];

    // Only show the followers and blocks for that user
    if let Some(pid) = person_id {
      if pid == person_details_id {
        follows = blocking(context.pool(), move |conn| {
          CommunityFollowerView::for_person(conn, person_details_id)
        })
        .await??;
        person_blocks = blocking(context.pool(), move |conn| {
          PersonBlockView::for_person(conn, person_details_id)
        })
        .await??;
        community_blocks = blocking(context.pool(), move |conn| {
          CommunityBlockView::for_person(conn, person_details_id)
        })
        .await??;
      }
    };

    let moderates = blocking(context.pool(), move |conn| {
      CommunityModeratorView::for_person(conn, person_details_id)
    })
    .await??;

    // Return the jwt
    Ok(GetPersonDetailsResponse {
      person_view,
      follows,
      community_blocks,
      person_blocks,
      moderates,
      comments,
      posts,
    })
  }
}

use crate::Perform;
use actix_web::web::Data;
use anyhow::Context;
use lemmy_api_common::{
  blocking,
  check_community_ban,
  community::*,
  get_local_user_view_from_jwt,
  is_mod_or_admin,
};
use lemmy_apub::{ActorType, CommunityType, UserType};
use lemmy_db_queries::{
  source::{comment::Comment_, community::CommunityModerator_, post::Post_},
  Bannable,
  Blockable,
  Crud,
  Followable,
  Joinable,
};
use lemmy_db_schema::source::{
  comment::Comment,
  community::*,
  community_block::{CommunityBlock, CommunityBlockForm},
  moderator::*,
  person::Person,
  post::Post,
  site::*,
};
use lemmy_db_views::comment_view::CommentQueryBuilder;
use lemmy_db_views_actor::{
  community_moderator_view::CommunityModeratorView,
  community_view::CommunityView,
  person_view::PersonViewSafe,
};
use lemmy_utils::{location_info, utils::naive_from_unix, ApiError, ConnectionId, LemmyError};
use lemmy_websocket::{messages::SendCommunityRoomMessage, LemmyContext, UserOperation};

#[async_trait::async_trait(?Send)]
impl Perform for FollowCommunity {
  type Response = CommunityResponse;

  async fn perform(
    &self,
    context: &Data<LemmyContext>,
    _websocket_id: Option<ConnectionId>,
  ) -> Result<CommunityResponse, LemmyError> {
    let data: &FollowCommunity = self;
    let local_user_view = get_local_user_view_from_jwt(&data.auth, context.pool()).await?;

    let community_id = data.community_id;
    let community = blocking(context.pool(), move |conn| {
      Community::read(conn, community_id)
    })
    .await??;
    let community_follower_form = CommunityFollowerForm {
      community_id: data.community_id,
      person_id: local_user_view.person.id,
      pending: false,
    };

    if community.local {
      if data.follow {
        check_community_ban(local_user_view.person.id, community_id, context.pool()).await?;

        let follow = move |conn: &'_ _| CommunityFollower::follow(conn, &community_follower_form);
        if blocking(context.pool(), follow).await?.is_err() {
          return Err(ApiError::err("community_follower_already_exists").into());
        }
      } else {
        let unfollow =
          move |conn: &'_ _| CommunityFollower::unfollow(conn, &community_follower_form);
        if blocking(context.pool(), unfollow).await?.is_err() {
          return Err(ApiError::err("community_follower_already_exists").into());
        }
      }
    } else if data.follow {
      // Dont actually add to the community followers here, because you need
      // to wait for the accept
      local_user_view
        .person
        .send_follow(&community.actor_id(), context)
        .await?;
    } else {
      local_user_view
        .person
        .send_unfollow(&community.actor_id(), context)
        .await?;
      let unfollow = move |conn: &'_ _| CommunityFollower::unfollow(conn, &community_follower_form);
      if blocking(context.pool(), unfollow).await?.is_err() {
        return Err(ApiError::err("community_follower_already_exists").into());
      }
    }

    let community_id = data.community_id;
    let person_id = local_user_view.person.id;
    let mut community_view = blocking(context.pool(), move |conn| {
      CommunityView::read(conn, community_id, Some(person_id))
    })
    .await??;

    // TODO: this needs to return a "pending" state, until Accept is received from the remote server
    // For now, just assume that remote follows are accepted.
    // Otherwise, the subscribed will be null
    if !community.local {
      community_view.subscribed = data.follow;
    }

    Ok(CommunityResponse { community_view })
  }
}

#[async_trait::async_trait(?Send)]
impl Perform for BlockCommunity {
  type Response = CommunityResponse;

  async fn perform(
    &self,
    context: &Data<LemmyContext>,
    _websocket_id: Option<ConnectionId>,
  ) -> Result<CommunityResponse, LemmyError> {
    let data: &BlockCommunity = &self;
    let local_user_view = get_local_user_view_from_jwt(&data.auth, context.pool()).await?;

    let community_id = data.community_id;
    let person_id = local_user_view.person.id;
    let community_block_form = CommunityBlockForm {
      person_id,
      community_id,
    };

    if data.block {
      let block = move |conn: &'_ _| CommunityBlock::block(conn, &community_block_form);
      if blocking(context.pool(), block).await?.is_err() {
        return Err(ApiError::err("community_block_already_exists").into());
      }
    } else {
      let unblock = move |conn: &'_ _| CommunityBlock::unblock(conn, &community_block_form);
      if blocking(context.pool(), unblock).await?.is_err() {
        return Err(ApiError::err("community_block_already_exists").into());
      }
    }

    // TODO does any federated stuff need to be done here?

    let community_view = blocking(context.pool(), move |conn| {
      CommunityView::read(conn, community_id, Some(person_id))
    })
    .await??;

    Ok(CommunityResponse { community_view })
  }
}

#[async_trait::async_trait(?Send)]
impl Perform for BanFromCommunity {
  type Response = BanFromCommunityResponse;

  async fn perform(
    &self,
    context: &Data<LemmyContext>,
    websocket_id: Option<ConnectionId>,
  ) -> Result<BanFromCommunityResponse, LemmyError> {
    let data: &BanFromCommunity = self;
    let local_user_view = get_local_user_view_from_jwt(&data.auth, context.pool()).await?;

    let community_id = data.community_id;
    let banned_person_id = data.person_id;

    // Verify that only mods or admins can ban
    is_mod_or_admin(context.pool(), local_user_view.person.id, community_id).await?;

    let community_user_ban_form = CommunityPersonBanForm {
      community_id: data.community_id,
      person_id: data.person_id,
    };

    let community = blocking(context.pool(), move |conn: &'_ _| {
      Community::read(conn, community_id)
    })
    .await??;
    let banned_person = blocking(context.pool(), move |conn: &'_ _| {
      Person::read(conn, banned_person_id)
    })
    .await??;

    if data.ban {
      let ban = move |conn: &'_ _| CommunityPersonBan::ban(conn, &community_user_ban_form);
      if blocking(context.pool(), ban).await?.is_err() {
        return Err(ApiError::err("community_user_already_banned").into());
      }

      // Also unsubscribe them from the community, if they are subscribed
      let community_follower_form = CommunityFollowerForm {
        community_id: data.community_id,
        person_id: banned_person_id,
        pending: false,
      };
      blocking(context.pool(), move |conn: &'_ _| {
        CommunityFollower::unfollow(conn, &community_follower_form)
      })
      .await?
      .ok();

      community
        .send_block_user(&local_user_view.person, banned_person, context)
        .await?;
    } else {
      let unban = move |conn: &'_ _| CommunityPersonBan::unban(conn, &community_user_ban_form);
      if blocking(context.pool(), unban).await?.is_err() {
        return Err(ApiError::err("community_user_already_banned").into());
      }
      community
        .send_undo_block_user(&local_user_view.person, banned_person, context)
        .await?;
    }

    // Remove/Restore their data if that's desired
    if data.remove_data.unwrap_or(false) {
      // Posts
      blocking(context.pool(), move |conn: &'_ _| {
        Post::update_removed_for_creator(conn, banned_person_id, Some(community_id), true)
      })
      .await??;

      // Comments
      // TODO Diesel doesn't allow updates with joins, so this has to be a loop
      let comments = blocking(context.pool(), move |conn| {
        CommentQueryBuilder::create(conn)
          .creator_id(banned_person_id)
          .community_id(community_id)
          .limit(std::i64::MAX)
          .list()
      })
      .await??;

      for comment_view in &comments {
        let comment_id = comment_view.comment.id;
        blocking(context.pool(), move |conn: &'_ _| {
          Comment::update_removed(conn, comment_id, true)
        })
        .await??;
      }
    }

    // Mod tables
    // TODO eventually do correct expires
    let expires = data.expires.map(naive_from_unix);

    let form = ModBanFromCommunityForm {
      mod_person_id: local_user_view.person.id,
      other_person_id: data.person_id,
      community_id: data.community_id,
      reason: data.reason.to_owned(),
      banned: Some(data.ban),
      expires,
    };
    blocking(context.pool(), move |conn| {
      ModBanFromCommunity::create(conn, &form)
    })
    .await??;

    let person_id = data.person_id;
    let person_view = blocking(context.pool(), move |conn| {
      PersonViewSafe::read(conn, person_id)
    })
    .await??;

    let res = BanFromCommunityResponse {
      person_view,
      banned: data.ban,
    };

    context.chat_server().do_send(SendCommunityRoomMessage {
      op: UserOperation::BanFromCommunity,
      response: res.clone(),
      community_id,
      websocket_id,
    });

    Ok(res)
  }
}

#[async_trait::async_trait(?Send)]
impl Perform for AddModToCommunity {
  type Response = AddModToCommunityResponse;

  async fn perform(
    &self,
    context: &Data<LemmyContext>,
    websocket_id: Option<ConnectionId>,
  ) -> Result<AddModToCommunityResponse, LemmyError> {
    let data: &AddModToCommunity = self;
    let local_user_view = get_local_user_view_from_jwt(&data.auth, context.pool()).await?;

    let community_id = data.community_id;

    // Verify that only mods or admins can add mod
    is_mod_or_admin(context.pool(), local_user_view.person.id, community_id).await?;

    // Update in local database
    let community_moderator_form = CommunityModeratorForm {
      community_id: data.community_id,
      person_id: data.person_id,
    };
    if data.added {
      let join = move |conn: &'_ _| CommunityModerator::join(conn, &community_moderator_form);
      if blocking(context.pool(), join).await?.is_err() {
        return Err(ApiError::err("community_moderator_already_exists").into());
      }
    } else {
      let leave = move |conn: &'_ _| CommunityModerator::leave(conn, &community_moderator_form);
      if blocking(context.pool(), leave).await?.is_err() {
        return Err(ApiError::err("community_moderator_already_exists").into());
      }
    }

    // Mod tables
    let form = ModAddCommunityForm {
      mod_person_id: local_user_view.person.id,
      other_person_id: data.person_id,
      community_id: data.community_id,
      removed: Some(!data.added),
    };
    blocking(context.pool(), move |conn| {
      ModAddCommunity::create(conn, &form)
    })
    .await??;

    // Send to federated instances
    let updated_mod_id = data.person_id;
    let updated_mod = blocking(context.pool(), move |conn| {
      Person::read(conn, updated_mod_id)
    })
    .await??;
    let community = blocking(context.pool(), move |conn| {
      Community::read(conn, community_id)
    })
    .await??;
    if data.added {
      community
        .send_add_mod(&local_user_view.person, updated_mod, context)
        .await?;
    } else {
      community
        .send_remove_mod(&local_user_view.person, updated_mod, context)
        .await?;
    }

    // Note: in case a remote mod is added, this returns the old moderators list, it will only get
    //       updated once we receive an activity from the community (like `Announce/Add/Moderator`)
    let community_id = data.community_id;
    let moderators = blocking(context.pool(), move |conn| {
      CommunityModeratorView::for_community(conn, community_id)
    })
    .await??;

    let res = AddModToCommunityResponse { moderators };
    context.chat_server().do_send(SendCommunityRoomMessage {
      op: UserOperation::AddModToCommunity,
      response: res.clone(),
      community_id,
      websocket_id,
    });
    Ok(res)
  }
}

// TODO: we dont do anything for federation here, it should be updated the next time the community
//       gets fetched. i hope we can get rid of the community creator role soon.
#[async_trait::async_trait(?Send)]
impl Perform for TransferCommunity {
  type Response = GetCommunityResponse;

  async fn perform(
    &self,
    context: &Data<LemmyContext>,
    _websocket_id: Option<ConnectionId>,
  ) -> Result<GetCommunityResponse, LemmyError> {
    let data: &TransferCommunity = self;
    let local_user_view = get_local_user_view_from_jwt(&data.auth, context.pool()).await?;

    let site_creator_id = blocking(context.pool(), move |conn| {
      Site::read(conn, 1).map(|s| s.creator_id)
    })
    .await??;

    let mut admins = blocking(context.pool(), move |conn| PersonViewSafe::admins(conn)).await??;

    // Making sure the site creator, if an admin, is at the top
    let creator_index = admins
      .iter()
      .position(|r| r.person.id == site_creator_id)
      .context(location_info!())?;
    let creator_person = admins.remove(creator_index);
    admins.insert(0, creator_person);

    // Fetch the community mods
    let community_id = data.community_id;
    let mut community_mods = blocking(context.pool(), move |conn| {
      CommunityModeratorView::for_community(conn, community_id)
    })
    .await??;

    // Make sure transferrer is either the top community mod, or an admin
    if local_user_view.person.id != community_mods[0].moderator.id
      && !admins
        .iter()
        .map(|a| a.person.id)
        .any(|x| x == local_user_view.person.id)
    {
      return Err(ApiError::err("not_an_admin").into());
    }

    // You have to re-do the community_moderator table, reordering it.
    // Add the transferee to the top
    let creator_index = community_mods
      .iter()
      .position(|r| r.moderator.id == data.person_id)
      .context(location_info!())?;
    let creator_person = community_mods.remove(creator_index);
    community_mods.insert(0, creator_person);

    // Delete all the mods
    let community_id = data.community_id;
    blocking(context.pool(), move |conn| {
      CommunityModerator::delete_for_community(conn, community_id)
    })
    .await??;

    // TODO: this should probably be a bulk operation
    // Re-add the mods, in the new order
    for cmod in &community_mods {
      let community_moderator_form = CommunityModeratorForm {
        community_id: cmod.community.id,
        person_id: cmod.moderator.id,
      };

      let join = move |conn: &'_ _| CommunityModerator::join(conn, &community_moderator_form);
      if blocking(context.pool(), join).await?.is_err() {
        return Err(ApiError::err("community_moderator_already_exists").into());
      }
    }

    // Mod tables
    // TODO there should probably be another table for transfer community
    // Right now, it will just look like it modded them twice
    let form = ModAddCommunityForm {
      mod_person_id: local_user_view.person.id,
      other_person_id: data.person_id,
      community_id: data.community_id,
      removed: Some(false),
    };
    blocking(context.pool(), move |conn| {
      ModAddCommunity::create(conn, &form)
    })
    .await??;

    let community_id = data.community_id;
    let person_id = local_user_view.person.id;
    let community_view = blocking(context.pool(), move |conn| {
      CommunityView::read(conn, community_id, Some(person_id))
    })
    .await?
    .map_err(|_| ApiError::err("couldnt_find_community"))?;

    let community_id = data.community_id;
    let moderators = blocking(context.pool(), move |conn| {
      CommunityModeratorView::for_community(conn, community_id)
    })
    .await?
    .map_err(|_| ApiError::err("couldnt_find_community"))?;

    // Return the jwt
    Ok(GetCommunityResponse {
      community_view,
      moderators,
      online: 0,
    })
  }
}

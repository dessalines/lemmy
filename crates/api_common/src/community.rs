use lemmy_db_schema::{CommunityId, PersonId};
use lemmy_db_views_actor::{
  community_block_view::CommunityBlockView,
  community_follower_view::CommunityFollowerView,
  community_moderator_view::CommunityModeratorView,
  community_view::CommunityView,
  person_view::PersonViewSafe,
};
use serde::{Deserialize, Serialize};

#[derive(Deserialize)]
pub struct GetCommunity {
  pub id: Option<CommunityId>,
  pub name: Option<String>,
  pub auth: Option<String>,
}

#[derive(Serialize)]
pub struct GetCommunityResponse {
  pub community_view: CommunityView,
  pub moderators: Vec<CommunityModeratorView>,
  pub online: usize,
}

#[derive(Deserialize)]
pub struct CreateCommunity {
  pub name: String,
  pub title: String,
  pub description: Option<String>,
  pub icon: Option<String>,
  pub banner: Option<String>,
  pub nsfw: Option<bool>,
  pub auth: String,
}

#[derive(Serialize, Clone)]
pub struct CommunityResponse {
  pub community_view: CommunityView,
}

#[derive(Deserialize, Debug)]
pub struct ListCommunities {
  pub type_: Option<String>,
  pub sort: Option<String>,
  pub page: Option<i64>,
  pub limit: Option<i64>,
  pub auth: Option<String>,
}

#[derive(Serialize, Debug)]
pub struct ListCommunitiesResponse {
  pub communities: Vec<CommunityView>,
}

#[derive(Deserialize, Clone)]
pub struct BanFromCommunity {
  pub community_id: CommunityId,
  pub person_id: PersonId,
  pub ban: bool,
  pub remove_data: Option<bool>,
  pub reason: Option<String>,
  pub expires: Option<i64>,
  pub auth: String,
}

#[derive(Serialize, Clone)]
pub struct BanFromCommunityResponse {
  pub person_view: PersonViewSafe,
  pub banned: bool,
}

#[derive(Deserialize)]
pub struct AddModToCommunity {
  pub community_id: CommunityId,
  pub person_id: PersonId,
  pub added: bool,
  pub auth: String,
}

#[derive(Serialize, Clone)]
pub struct AddModToCommunityResponse {
  pub moderators: Vec<CommunityModeratorView>,
}

#[derive(Deserialize)]
pub struct EditCommunity {
  pub community_id: CommunityId,
  pub title: Option<String>,
  pub description: Option<String>,
  pub icon: Option<String>,
  pub banner: Option<String>,
  pub nsfw: Option<bool>,
  pub auth: String,
}

#[derive(Deserialize)]
pub struct DeleteCommunity {
  pub community_id: CommunityId,
  pub deleted: bool,
  pub auth: String,
}

#[derive(Deserialize)]
pub struct RemoveCommunity {
  pub community_id: CommunityId,
  pub removed: bool,
  pub reason: Option<String>,
  pub expires: Option<i64>,
  pub auth: String,
}

#[derive(Deserialize)]
pub struct FollowCommunity {
  pub community_id: CommunityId,
  pub follow: bool,
  pub auth: String,
}

#[derive(Deserialize)]
pub struct BlockCommunity {
  pub community_id: CommunityId,
  pub block: bool,
  pub auth: String,
}

#[derive(Deserialize)]
pub struct GetFollowedCommunities {
  pub auth: String,
}

#[derive(Serialize)]
pub struct GetFollowedCommunitiesResponse {
  pub communities: Vec<CommunityFollowerView>,
}

#[derive(Deserialize)]
pub struct GetBlockedCommunities {
  pub auth: String,
}

#[derive(Serialize)]
pub struct GetBlockedCommunitiesResponse {
  pub communities: Vec<CommunityBlockView>,
}

#[derive(Deserialize)]
pub struct TransferCommunity {
  pub community_id: CommunityId,
  pub person_id: PersonId,
  pub auth: String,
}

use crate::{
  activities::{
    comment::{
      create::CreateComment,
      delete::DeleteComment,
      dislike::DislikeComment,
      like::LikeComment,
      remove::RemoveComment,
      undo_delete::UndoDeleteComment,
      undo_dislike::UndoDislikeComment,
      undo_like::UndoLikeComment,
      undo_remove::UndoRemoveComment,
      update::UpdateComment,
    },
    community::{block_user::BlockUserFromCommunity, undo_block_user::UndoBlockUserFromCommunity},
    post::{
      create::CreatePost,
      delete::DeletePost,
      dislike::DislikePost,
      like::LikePost,
      remove::RemovePost,
      undo_delete::UndoDeletePost,
      undo_dislike::UndoDislikePost,
      undo_like::UndoLikePost,
      undo_remove::UndoRemovePost,
      update::UpdatePost,
    },
  },
  http::is_activity_already_known,
};
use activitystreams::activity::kind::RemoveType;
use lemmy_apub::check_is_apub_id_valid;
use lemmy_apub_lib::{verify_domains_match, ActivityCommonFields, ActivityHandlerNew, PublicUrl};
use lemmy_utils::LemmyError;
use lemmy_websocket::LemmyContext;
use serde::{Deserialize, Serialize};
use url::Url;

#[derive(Clone, Debug, Deserialize, Serialize, ActivityHandlerNew)]
pub enum AnnouncableActivities {
  CreateComment(CreateComment),
  UpdateComment(UpdateComment),
  LikeComment(LikeComment),
  DislikeComment(DislikeComment),
  UndoLikeComment(UndoLikeComment),
  UndoDislikeComment(UndoDislikeComment),
  DeleteComment(DeleteComment),
  UndoDeleteComment(UndoDeleteComment),
  RemoveComment(RemoveComment),
  UndoRemoveComment(UndoRemoveComment),
  CreatePost(CreatePost),
  UpdatePost(UpdatePost),
  LikePost(LikePost),
  DislikePost(DislikePost),
  DeletePost(DeletePost),
  UndoDeletePost(UndoDeletePost),
  RemovePost(RemovePost),
  UndoRemovePost(UndoRemovePost),
  UndoLikePost(UndoLikePost),
  UndoDislikePost(UndoDislikePost),
  BlockUserFromCommunity(BlockUserFromCommunity),
  UndoBlockUserFromCommunity(UndoBlockUserFromCommunity),
}

#[derive(Clone, Debug, serde::Deserialize, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AnnounceActivity {
  to: PublicUrl,
  object: AnnouncableActivities,
  cc: [Url; 1],
  #[serde(rename = "type")]
  kind: RemoveType,
  #[serde(flatten)]
  common: ActivityCommonFields,
}

#[async_trait::async_trait(?Send)]
impl ActivityHandlerNew for AnnounceActivity {
  async fn verify(
    &self,
    context: &LemmyContext,
    request_counter: &mut i32,
  ) -> Result<(), LemmyError> {
    verify_domains_match(&self.common.actor, self.common.id_unchecked())?;
    verify_domains_match(&self.common.actor, &self.cc[0])?;
    check_is_apub_id_valid(&self.common.actor, false)?;
    self.object.verify(context, request_counter).await
  }

  async fn receive(
    &self,
    context: &LemmyContext,
    request_counter: &mut i32,
  ) -> Result<(), LemmyError> {
    if is_activity_already_known(context.pool(), self.object.common().id_unchecked()).await? {
      return Ok(());
    }
    self.object.receive(context, request_counter).await
  }

  fn common(&self) -> &ActivityCommonFields {
    &self.common
  }
}

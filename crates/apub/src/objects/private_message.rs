use crate::{
  extensions::{context::lemmy_context, note_extension::NoteExtension},
  fetcher::person::get_or_fetch_and_upsert_person,
  objects::{
    check_object_domain,
    create_tombstone,
    get_object_from_apub,
    get_source_markdown_value,
    set_content_and_source,
    FromApub,
    FromApubToForm,
    ToApub,
  },
  NoteExt,
};
use activitystreams::{
  object::{kind::NoteType, ApObject, Note, Tombstone},
  prelude::*,
};
use activitystreams_ext::Ext1;
use anyhow::Context;
use lemmy_api_common::blocking;
use lemmy_db_queries::{Crud, DbPool};
use lemmy_db_schema::source::{
  person::Person,
  private_message::{PrivateMessage, PrivateMessageForm},
};
use lemmy_utils::{location_info, utils::convert_datetime, LemmyError};
use lemmy_websocket::LemmyContext;
use url::Url;

#[async_trait::async_trait(?Send)]
impl ToApub for PrivateMessage {
  type ApubType = NoteExt;

  async fn to_apub(&self, pool: &DbPool) -> Result<NoteExt, LemmyError> {
    let mut private_message = ApObject::new(Note::new());

    let creator_id = self.creator_id;
    let creator = blocking(pool, move |conn| Person::read(conn, creator_id)).await??;

    let recipient_id = self.recipient_id;
    let recipient = blocking(pool, move |conn| Person::read(conn, recipient_id)).await??;

    private_message
      .set_many_contexts(lemmy_context()?)
      .set_id(self.ap_id.to_owned().into_inner())
      .set_published(convert_datetime(self.published))
      .set_to(recipient.actor_id.into_inner())
      .set_attributed_to(creator.actor_id.into_inner());

    set_content_and_source(&mut private_message, &self.content)?;

    if let Some(u) = self.updated {
      private_message.set_updated(convert_datetime(u));
    }

    let ext = NoteExtension::new(self.language.to_owned())?;
    Ok(Ext1::new(private_message, ext))
  }

  fn to_tombstone(&self) -> Result<Tombstone, LemmyError> {
    create_tombstone(
      self.deleted,
      self.ap_id.to_owned().into(),
      self.updated,
      NoteType::Note,
    )
  }
}

#[async_trait::async_trait(?Send)]
impl FromApub for PrivateMessage {
  type ApubType = NoteExt;

  async fn from_apub(
    note: &NoteExt,
    context: &LemmyContext,
    expected_domain: Url,
    request_counter: &mut i32,
    mod_action_allowed: bool,
  ) -> Result<PrivateMessage, LemmyError> {
    get_object_from_apub(
      note,
      context,
      expected_domain,
      request_counter,
      mod_action_allowed,
    )
    .await
  }
}

#[async_trait::async_trait(?Send)]
impl FromApubToForm<NoteExt> for PrivateMessageForm {
  async fn from_apub(
    note: &NoteExt,
    context: &LemmyContext,
    expected_domain: Url,
    request_counter: &mut i32,
    _mod_action_allowed: bool,
  ) -> Result<PrivateMessageForm, LemmyError> {
    let creator_actor_id = note
      .attributed_to()
      .context(location_info!())?
      .clone()
      .single_xsd_any_uri()
      .context(location_info!())?;

    let creator =
      get_or_fetch_and_upsert_person(&creator_actor_id, context, request_counter).await?;
    let recipient_actor_id = note
      .to()
      .context(location_info!())?
      .clone()
      .single_xsd_any_uri()
      .context(location_info!())?;
    let recipient =
      get_or_fetch_and_upsert_person(&recipient_actor_id, context, request_counter).await?;
    let ap_id = Some(check_object_domain(note, expected_domain, false)?);

    let content = get_source_markdown_value(note)?.context(location_info!())?;

    Ok(PrivateMessageForm {
      creator_id: creator.id,
      recipient_id: recipient.id,
      content,
      published: note.published().map(|u| u.to_owned().naive_local()),
      updated: note.updated().map(|u| u.to_owned().naive_local()),
      deleted: None,
      read: None,
      ap_id,
      local: Some(false),
      language: note.ext_one.language.to_owned(),
    })
  }
}

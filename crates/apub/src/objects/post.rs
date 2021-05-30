use crate::{
  check_is_apub_id_valid,
  extensions::{context::lemmy_context, page_extension::PageExtension},
  fetcher::person::get_or_fetch_and_upsert_person,
  get_community_from_to_or_cc,
  objects::{
    check_object_domain,
    check_object_for_community_or_site_ban,
    create_tombstone,
    get_object_from_apub,
    get_source_markdown_value,
    set_content_and_source,
    FromApub,
    FromApubToForm,
    ToApub,
  },
  PageExt,
};
use activitystreams::{
  object::{kind::PageType, ApObject, Image, Page, Tombstone},
  prelude::*,
  public,
};
use activitystreams_ext::Ext1;
use anyhow::Context;
use lemmy_api_common::blocking;
use lemmy_db_queries::{Crud, DbPool};
use lemmy_db_schema::{
  self,
  source::{
    community::Community,
    person::Person,
    post::{Post, PostForm},
  },
};
use lemmy_utils::{
  location_info,
  request::fetch_iframely_and_pictrs_data,
  utils::{check_slurs, convert_datetime, remove_slurs},
  LemmyError,
};
use lemmy_websocket::LemmyContext;
use url::Url;

#[async_trait::async_trait(?Send)]
impl ToApub for Post {
  type ApubType = PageExt;

  // Turn a Lemmy post into an ActivityPub page that can be sent out over the network.
  async fn to_apub(&self, pool: &DbPool) -> Result<PageExt, LemmyError> {
    let mut page = ApObject::new(Page::new());

    let creator_id = self.creator_id;
    let creator = blocking(pool, move |conn| Person::read(conn, creator_id)).await??;

    let community_id = self.community_id;
    let community = blocking(pool, move |conn| Community::read(conn, community_id)).await??;

    page
      // Not needed when the Post is embedded in a collection (like for community outbox)
      // TODO: need to set proper context defining sensitive/commentsEnabled fields
      // https://git.asonix.dog/Aardwolf/activitystreams/issues/5
      .set_many_contexts(lemmy_context()?)
      .set_id(self.ap_id.to_owned().into_inner())
      .set_name(self.name.to_owned())
      // `summary` field for compatibility with lemmy v0.9.9 and older,
      // TODO: remove this after some time
      .set_summary(self.name.to_owned())
      .set_published(convert_datetime(self.published))
      .set_many_tos(vec![community.actor_id.into_inner(), public()])
      .set_attributed_to(creator.actor_id.into_inner());

    if let Some(body) = &self.body {
      set_content_and_source(&mut page, &body)?;
    }

    if let Some(url) = &self.url {
      page.set_url::<Url>(url.to_owned().into());
    }

    if let Some(thumbnail_url) = &self.thumbnail_url {
      let mut image = Image::new();
      image.set_url::<Url>(thumbnail_url.to_owned().into());
      page.set_image(image.into_any_base()?);
    }

    if let Some(u) = self.updated {
      page.set_updated(convert_datetime(u));
    }

    let ext = PageExtension::new(
      !self.locked,
      self.nsfw,
      self.stickied,
      self.language.to_owned(),
    )?;
    Ok(Ext1::new(page, ext))
  }

  fn to_tombstone(&self) -> Result<Tombstone, LemmyError> {
    create_tombstone(
      self.deleted,
      self.ap_id.to_owned().into(),
      self.updated,
      PageType::Page,
    )
  }
}

#[async_trait::async_trait(?Send)]
impl FromApub for Post {
  type ApubType = PageExt;

  /// Converts a `PageExt` to `PostForm`.
  ///
  /// If the post's community or creator are not known locally, these are also fetched.
  async fn from_apub(
    page: &PageExt,
    context: &LemmyContext,
    expected_domain: Url,
    request_counter: &mut i32,
    mod_action_allowed: bool,
  ) -> Result<Post, LemmyError> {
    let post: Post = get_object_from_apub(
      page,
      context,
      expected_domain,
      request_counter,
      mod_action_allowed,
    )
    .await?;
    check_object_for_community_or_site_ban(page, post.community_id, context, request_counter)
      .await?;
    Ok(post)
  }
}

#[async_trait::async_trait(?Send)]
impl FromApubToForm<PageExt> for PostForm {
  async fn from_apub(
    page: &PageExt,
    context: &LemmyContext,
    expected_domain: Url,
    request_counter: &mut i32,
    mod_action_allowed: bool,
  ) -> Result<PostForm, LemmyError> {
    let community = get_community_from_to_or_cc(page, context, request_counter).await?;
    let ap_id = if mod_action_allowed {
      let id = page.id_unchecked().context(location_info!())?;
      check_is_apub_id_valid(id, community.local)?;
      id.to_owned().into()
    } else {
      check_object_domain(page, expected_domain, community.local)?
    };
    let ext = &page.ext_one;
    let creator_actor_id = page
      .inner
      .attributed_to()
      .as_ref()
      .context(location_info!())?
      .as_single_xsd_any_uri()
      .context(location_info!())?;

    let creator =
      get_or_fetch_and_upsert_person(creator_actor_id, context, request_counter).await?;

    let thumbnail_url: Option<Url> = match &page.inner.image() {
      Some(any_image) => Image::from_any_base(
        any_image
          .to_owned()
          .as_one()
          .context(location_info!())?
          .to_owned(),
      )?
      .context(location_info!())?
      .url()
      .context(location_info!())?
      .as_single_xsd_any_uri()
      .map(|url| url.to_owned()),
      None => None,
    };
    let url = page
      .inner
      .url()
      .map(|u| u.as_single_xsd_any_uri())
      .flatten()
      .map(|u| u.to_owned());

    let (iframely_title, iframely_description, iframely_html, pictrs_thumbnail) =
      if let Some(url) = &url {
        fetch_iframely_and_pictrs_data(context.client(), Some(url)).await
      } else {
        (None, None, None, thumbnail_url)
      };

    let name = page
      .inner
      .name()
      // The following is for compatibility with lemmy v0.9.9 and older
      // TODO: remove it after some time (along with the map above)
      .or_else(|| page.inner.summary())
      .context(location_info!())?
      .as_single_xsd_string()
      .context(location_info!())?
      .to_string();
    let body = get_source_markdown_value(page)?;

    // TODO: expected_domain is wrong in this case, because it simply takes the domain of the actor
    //       maybe we need to take id_unchecked() if the activity is from community to user?
    //       why did this work before? -> i dont think it did?
    //       -> try to make expected_domain optional and set it null if it is a mod action

    check_slurs(&name)?;
    let body_slurs_removed = body.map(|b| remove_slurs(&b));
    Ok(PostForm {
      name,
      url: url.map(|u| u.into()),
      body: body_slurs_removed,
      creator_id: creator.id,
      community_id: community.id,
      removed: None,
      locked: ext.comments_enabled.map(|e| !e),
      published: page
        .inner
        .published()
        .as_ref()
        .map(|u| u.to_owned().naive_local()),
      updated: page
        .inner
        .updated()
        .as_ref()
        .map(|u| u.to_owned().naive_local()),
      deleted: None,
      nsfw: ext.sensitive,
      stickied: ext.stickied,
      embed_title: iframely_title,
      embed_description: iframely_description,
      embed_html: iframely_html,
      thumbnail_url: pictrs_thumbnail.map(|u| u.into()),
      ap_id: Some(ap_id),
      local: Some(false),
      language: ext.language.to_owned(),
    })
  }
}

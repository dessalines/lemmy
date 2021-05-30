use crate::PerformCrud;
use actix_web::web::Data;
use lemmy_api_common::{
  blocking,
  build_federated_instances,
  get_local_user_settings_view_from_jwt_opt,
  person::Register,
  site::*,
};
use lemmy_db_schema::PrimaryLanguageTag;
use lemmy_db_views::site_view::SiteView;
use lemmy_db_views_actor::person_view::PersonViewSafe;
use lemmy_utils::{settings::structs::Settings, version, ConnectionId, LemmyError};
use lemmy_websocket::{messages::GetUsersOnline, LemmyContext};
use log::info;

#[async_trait::async_trait(?Send)]
impl PerformCrud for GetSite {
  type Response = GetSiteResponse;

  async fn perform(
    &self,
    context: &Data<LemmyContext>,
    websocket_id: Option<ConnectionId>,
  ) -> Result<GetSiteResponse, LemmyError> {
    let data: &GetSite = &self;

    let site_view = match blocking(context.pool(), move |conn| SiteView::read(conn)).await? {
      Ok(site_view) => Some(site_view),
      // If the site isn't created yet, check the setup
      Err(_) => {
        if let Some(setup) = Settings::get().setup().as_ref() {
          let register = Register {
            username: setup.admin_username.to_owned(),
            email: setup.admin_email.to_owned(),
            password: setup.admin_password.to_owned(),
            password_verify: setup.admin_password.to_owned(),
            show_nsfw: true,
            captcha_uuid: None,
            captcha_answer: None,
            discussion_languages: vec![],
          };
          let login_response = register.perform(context, websocket_id).await?;
          info!("Admin {} created", setup.admin_username);

          let create_site = CreateSite {
            name: setup.site_name.to_owned(),
            sidebar: None,
            description: None,
            icon: None,
            banner: None,
            enable_downvotes: None,
            open_registration: None,
            enable_nsfw: None,
            auth: login_response.jwt,
            community_creation_admin_only: None,
          };
          create_site.perform(context, websocket_id).await?;
          info!("Site {} created", setup.site_name);
          Some(blocking(context.pool(), move |conn| SiteView::read(conn)).await??)
        } else {
          None
        }
      }
    };

    let mut admins = blocking(context.pool(), move |conn| PersonViewSafe::admins(conn)).await??;

    // Make sure the site creator is the top admin
    if let Some(site_view) = site_view.to_owned() {
      let site_creator_id = site_view.creator.id;
      // TODO investigate why this is sometimes coming back null
      // Maybe user_.admin isn't being set to true?
      if let Some(creator_index) = admins.iter().position(|r| r.person.id == site_creator_id) {
        let creator_person = admins.remove(creator_index);
        admins.insert(0, creator_person);
      }
    }

    let banned = blocking(context.pool(), move |conn| PersonViewSafe::banned(conn)).await??;

    let online = context
      .chat_server()
      .send(GetUsersOnline)
      .await
      .unwrap_or(1);

    let my_user = get_local_user_settings_view_from_jwt_opt(&data.auth, context.pool()).await?;
    let federated_instances = build_federated_instances(context.pool()).await?;

    Ok(GetSiteResponse {
      site_view,
      admins,
      banned,
      online,
      version: version::VERSION.to_string(),
      my_user,
      federated_instances,
    })
  }
}

#[async_trait::async_trait(?Send)]
impl PerformCrud for GetLanguages {
  type Response = LanguagesResponse;

  async fn perform(
    &self,
    _context: &Data<LemmyContext>,
    _websocket_id: Option<ConnectionId>,
  ) -> Result<LanguagesResponse, LemmyError> {
    // TODO: get an actual language list
    // https://github.com/pyfisch/rust-language-tags/issues/32
    let languages = vec!["en", "es"]
      .iter()
      .map(|l| PrimaryLanguageTag(l.to_string()))
      .collect();
    Ok(LanguagesResponse { languages })
  }
}

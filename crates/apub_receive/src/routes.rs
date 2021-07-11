use crate::http::{
  comment::get_apub_comment,
  community::{
    community_inbox,
    get_apub_community_followers,
    get_apub_community_http,
    get_apub_community_inbox,
    get_apub_community_moderators,
    get_apub_community_outbox,
  },
  get_activity,
  person::{get_apub_person_http, get_apub_person_inbox, get_apub_person_outbox, person_inbox},
  post::get_apub_post,
  shared_inbox,
};
use actix_web::*;
use http_signature_normalization_actix::digest::middleware::VerifyDigest;
use lemmy_apub::APUB_JSON_CONTENT_TYPE;
use lemmy_utils::settings::structs::Settings;
use sha2::{Digest, Sha256};

static APUB_JSON_CONTENT_TYPE_LONG: &str =
  "application/ld+json; profile=\"https://www.w3.org/ns/activitystreams\"";

pub fn config(cfg: &mut web::ServiceConfig) {
  if Settings::get().federation().enabled {
    println!("federation enabled, host is {}", Settings::get().hostname());
    let digest_verifier = VerifyDigest::new(Sha256::new());

    let header_guard_accept = guard::Any(guard::Header("Accept", APUB_JSON_CONTENT_TYPE))
      .or(guard::Header("Accept", APUB_JSON_CONTENT_TYPE_LONG));
    let header_guard_content_type =
      guard::Any(guard::Header("Content-Type", APUB_JSON_CONTENT_TYPE))
        .or(guard::Header("Content-Type", APUB_JSON_CONTENT_TYPE_LONG));

    cfg
      .service(
        web::scope("/")
          .guard(header_guard_accept)
          .route(
            "/c/{community_name}",
            web::get().to(get_apub_community_http),
          )
          .route(
            "/c/{community_name}/followers",
            web::get().to(get_apub_community_followers),
          )
          .route(
            "/c/{community_name}/outbox",
            web::get().to(get_apub_community_outbox),
          )
          .route(
            "/c/{community_name}/inbox",
            web::get().to(get_apub_community_inbox),
          )
          .route(
            "/c/{community_name}/moderators",
            web::get().to(get_apub_community_moderators),
          )
          .route("/u/{user_name}", web::get().to(get_apub_person_http))
          .route(
            "/u/{user_name}/outbox",
            web::get().to(get_apub_person_outbox),
          )
          .route("/u/{user_name}/inbox", web::get().to(get_apub_person_inbox))
          .route("/post/{post_id}", web::get().to(get_apub_post))
          .route("/comment/{comment_id}", web::get().to(get_apub_comment))
          .route("/activities/{type_}/{id}", web::get().to(get_activity)),
      )
      // Inboxes dont work with the header guard for some reason.
      .service(
        web::scope("/")
          .wrap(digest_verifier)
          .guard(header_guard_content_type)
          .route("/c/{community_name}/inbox", web::post().to(community_inbox))
          .route("/u/{user_name}/inbox", web::post().to(person_inbox))
          .route("/inbox", web::post().to(shared_inbox)),
      );
  }
}

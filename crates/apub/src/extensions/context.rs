use activitystreams::{base::AnyBase, context};
use lemmy_utils::LemmyError;
use serde_json::json;
use url::Url;

pub fn lemmy_context() -> Result<Vec<AnyBase>, LemmyError> {
  let context_ext = AnyBase::from_arbitrary_json(json!(
  {
    "sc": "http://schema.org#",
    "sensitive": "as:sensitive",
    "stickied": "as:stickied",
    "pt": "https://join.lemmy.ml#",
    "commentsEnabled": {
      "type": "sc:Boolean",
      "id": "pt:commentsEnabled"
    },
    "moderators": "as:moderators",
    "matrixUserId": {
      "type": "sc:Text",
      "id": "as:alsoKnownAs"
    },
    "languages": {
      "type": "sc:Text"
    },
  }))?;
  Ok(vec![
    AnyBase::from(context()),
    context_ext,
    AnyBase::from(Url::parse("https://w3id.org/security/v1")?),
  ])
}

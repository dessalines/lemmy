use activitystreams::unparsed::UnparsedMutExt;
use activitystreams_ext::UnparsedExtension;
use lemmy_db_schema::PrimaryLanguageTag;
use lemmy_utils::LemmyError;
use serde::{Deserialize, Serialize};

/// Activitystreams extension to allow (de)serializing additional Post fields
/// `comemnts_enabled` (called 'locked' in Lemmy),
/// `sensitive` (called 'nsfw') and `stickied`.
#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NoteExtension {
  pub language: Option<PrimaryLanguageTag>,
}

impl NoteExtension {
  pub fn new(language: PrimaryLanguageTag) -> Result<NoteExtension, LemmyError> {
    Ok(NoteExtension {
      language: Some(language),
    })
  }
}

impl<U> UnparsedExtension<U> for NoteExtension
where
  U: UnparsedMutExt,
{
  type Error = serde_json::Error;

  fn try_from_unparsed(unparsed_mut: &mut U) -> Result<Self, Self::Error> {
    Ok(NoteExtension {
      language: unparsed_mut.remove("language")?,
    })
  }

  fn try_into_unparsed(self, unparsed_mut: &mut U) -> Result<(), Self::Error> {
    unparsed_mut.insert("language", self.language)?;
    Ok(())
  }
}

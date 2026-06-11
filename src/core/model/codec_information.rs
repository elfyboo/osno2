use crate::core::model::library_track::LibraryTrackType;
use symphonia::core::codecs::CodecId;

pub struct CodecInformation {
    pub codec_id: CodecId,
    pub track_type: LibraryTrackType,
    pub str_codec_id: &'static str,
    pub str_track_type: &'static str,
}

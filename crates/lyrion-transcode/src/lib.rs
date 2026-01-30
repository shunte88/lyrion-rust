//! Transcoding pipeline management
//! Ported from Slim/Player/TranscodingHelper.pm and convert.conf

pub use pipeline::TranscodePipeline;
pub use utils::{needs_transcoding, get_format_from_path};

mod pipeline;
mod utils;

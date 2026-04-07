use cidre::{arc, cm, sc};
use std::sync::mpsc;

use crate::MacScreenCapture;

impl MacScreenCapture {
    /// Receive the next raw sample buffer, checking the error flag between
    /// attempts. Useful for callers that want to process sample buffers
    /// directly rather than going through [`CaptureStream::next_frame`].
    pub fn get_next_sample_buffer(
        &self,
    ) -> Result<(arc::R<cm::SampleBuf>, sc::stream::OutputType), mpsc::RecvError> {
        self.next_raw_sample()
    }
}

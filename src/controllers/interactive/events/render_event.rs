use crate::controllers::interactive::data::frame_data::FrameData;
use crate::controllers::interactive::errors::render_error::RenderError;

#[derive(Debug)]
pub enum RenderEvent {
    Frame(FrameData),
    Error(RenderError),
}

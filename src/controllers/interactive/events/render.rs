use crate::controllers::interactive::data::frame_data::FrameData;
use crate::controllers::interactive::errors::render::RenderError;

#[derive(Debug)]
pub enum RenderEvent {
    Frame(FrameData),
    Error(RenderError),
}

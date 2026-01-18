use crate::controllers::interactive::events::render_event::RenderEvent;

pub trait FrameSink: Send + Sync {
    fn submit(&self, event: RenderEvent);
}

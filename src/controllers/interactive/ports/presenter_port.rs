use crate::controllers::interactive::events::render_event::RenderEvent;

pub trait PresenterPort: Send + Sync {
    fn present(&self, event: RenderEvent);
}

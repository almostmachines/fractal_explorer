use crate::controllers::interactive::events::render::RenderEvent;

pub trait InteractiveControllerPresenterPort: Send + Sync {
    fn present(&self, event: RenderEvent);
}

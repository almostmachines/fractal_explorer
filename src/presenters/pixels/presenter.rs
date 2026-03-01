use crate::controllers::interactive::data::frame_data::FrameData;
use crate::controllers::interactive::events::render::RenderEvent;
use crate::controllers::interactive::ports::presenter::InteractiveControllerPresenterPort;
use crate::input::gui::app::events::gui::GuiEvent;
use crate::input::gui::app::ports::presenter::GuiPresenterPort;
use crate::presenters::pixels::adapter::PixelsAdapter;
use egui::Context as EguiContext;
use egui_wgpu::Renderer as EguiRenderer;
use pixels::Pixels;
use pixels::SurfaceTexture;
use pixels::wgpu;
use std::sync::Arc;
use std::time::Duration;
use winit::event_loop::EventLoopProxy;
use winit::window::Window;

pub struct PixelsPresenter {
    pixels: Pixels<'static>,
    egui_renderer: EguiRenderer,
    adapter: Arc<PixelsAdapter>,
    width: u32,
    height: u32,
    has_frame: bool,
    last_presented_generation: u64,
    last_error_message: Option<String>,
    last_render_duration: Option<Duration>,
}

impl GuiPresenterPort for PixelsPresenter {
    fn new(window: &'static Window, event_loop_proxy: EventLoopProxy<GuiEvent>) -> Self {
        let size = window.inner_size();
        let surface_texture = SurfaceTexture::new(size.width, size.height, window);

        let pixels = Pixels::new(size.width, size.height, surface_texture)
            .expect("Failed to create pixels surface");

        let egui_renderer = EguiRenderer::new(
            pixels.device(),
            pixels.render_texture_format(),
            None, // depth format
            1,    // msaa samples
        );

        Self {
            pixels,
            egui_renderer,
            adapter: Arc::new(PixelsAdapter::new(event_loop_proxy)),
            width: size.width,
            height: size.height,
            has_frame: false,
            last_presented_generation: 0,
            last_error_message: None,
            last_render_duration: None,
        }
    }

    fn share_adapter(&self) -> Arc<dyn InteractiveControllerPresenterPort> {
        Arc::clone(&self.adapter) as Arc<dyn InteractiveControllerPresenterPort>
    }

    fn render(
        &mut self,
        egui_output: egui::FullOutput,
        egui_ctx: &EguiContext,
        _requested_generation: u64,
    ) -> Result<(), pixels::Error> {
        if self.width == 0 || self.height == 0 {
            return Ok(());
        }

        self.maybe_draw_frame();

        if !self.has_frame {
            self.draw_placeholder();
        }

        self.pixels.render_with(|encoder, render_target, context| {
            // First, render the pixels framebuffer (the scaling pass)
            context.scaling_renderer.render(encoder, render_target);

            let clipped_primitives =
                egui_ctx.tessellate(egui_output.shapes, egui_ctx.pixels_per_point());

            let screen_descriptor = egui_wgpu::ScreenDescriptor {
                size_in_pixels: [self.width, self.height],
                pixels_per_point: egui_ctx.pixels_per_point(),
            };

            let textures_delta = egui_output.textures_delta;

            // Upload new/changed egui textures
            for (id, delta) in &textures_delta.set {
                self.egui_renderer
                    .update_texture(&context.device, &context.queue, *id, delta);
            }

            // Update egui buffers (vertices, indices)
            self.egui_renderer.update_buffers(
                &context.device,
                &context.queue,
                encoder,
                &clipped_primitives,
                &screen_descriptor,
            );

            // Render egui on top of pixels framebuffer
            {
                let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                    label: Some("egui"),
                    color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                        view: render_target,
                        resolve_target: None,
                        ops: wgpu::Operations {
                            load: wgpu::LoadOp::Load, // Keep pixels content
                            store: wgpu::StoreOp::Store,
                        },
                    })],
                    depth_stencil_attachment: None,
                    ..Default::default()
                });

                self.egui_renderer.render(
                    &mut render_pass,
                    &clipped_primitives,
                    &screen_descriptor,
                );
            }

            // Free textures no longer needed
            for id in &textures_delta.free {
                self.egui_renderer.free_texture(id);
            }

            Ok(())
        })
    }

    fn resize(&mut self, width: u32, height: u32) {
        self.width = width;
        self.height = height;

        self.pixels
            .resize_surface(width, height)
            .expect("Failed to resize surface");

        self.pixels
            .resize_buffer(width, height)
            .expect("Failed to resize buffer");

        self.has_frame = false;
    }
}

impl PixelsPresenter {
    fn draw_placeholder(&mut self) {
        let frame = self.pixels.frame_mut();
        for pixel in frame.chunks_exact_mut(4) {
            pixel[0] = 0;
            pixel[1] = 0;
            pixel[2] = 0;
            pixel[3] = 255;
        }
    }

    pub fn maybe_draw_frame(&mut self) {
        if let Some(event) = self.adapter.render_event() {
            match event {
                RenderEvent::Frame(frame) => {
                    let pixel_rect = frame.pixel_buffer.pixel_rect();

                    if frame.generation > self.last_presented_generation
                        && pixel_rect.width() == self.width
                        && pixel_rect.height() == self.height
                    {
                        self.copy_pixel_buffer_into_pixels_frame(&frame);
                        self.has_frame = true;
                        self.last_presented_generation = frame.generation;
                        self.last_render_duration = Some(frame.render_duration);
                        self.last_error_message = None;
                    }
                }
                RenderEvent::Error(error) => {
                    if error.generation >= self.last_presented_generation {
                        self.last_error_message = Some(error.message);
                    }
                }
            }
        }
    }

    pub fn copy_pixel_buffer_into_pixels_frame(&mut self, frame: &FrameData) {
        let pixel_rect = frame.pixel_buffer.pixel_rect();
        let width = pixel_rect.width();
        let height = pixel_rect.height();
        let expected_rgba_len = (width * height * 4) as usize;
        let src = frame.pixel_buffer.buffer();
        let dest = self.pixels.frame_mut();

        assert_eq!(
            dest.len(),
            expected_rgba_len,
            "pixels frame length {} does not match expected {} for {}x{}",
            dest.len(),
            expected_rgba_len,
            width,
            height
        );

        for (src_pixel, dst_pixel) in src.chunks_exact(3).zip(dest.chunks_exact_mut(4)) {
            dst_pixel[0] = src_pixel[0];
            dst_pixel[1] = src_pixel[1];
            dst_pixel[2] = src_pixel[2];
            dst_pixel[3] = 255;
        }
    }
}

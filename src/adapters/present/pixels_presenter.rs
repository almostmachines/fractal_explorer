use std::sync::{Arc, Mutex};
use std::time::Duration;
use pixels::Pixels;
use pixels::SurfaceTexture;
use pixels::wgpu;
use egui::Context as EguiContext;
use egui_wgpu::Renderer as EguiRenderer;
use winit::event_loop::EventLoopProxy;
use winit::window::Window;
use crate::adapters::pixel_format::copy_rgb_to_rgba;
use crate::controllers::interactive::ports::presenter_port::PresenterPort;
use crate::controllers::interactive::data::frame_data::FrameData;
use crate::controllers::interactive::events::render_event::RenderEvent;
use crate::input::gui::GuiEvent;

struct PixelsPresenterPort {
    render_event: Mutex<Option<RenderEvent>>,
    event_loop_proxy: EventLoopProxy<GuiEvent>,
}

pub struct PixelsPresenter {
    pixels: Pixels<'static>,
    egui_renderer: EguiRenderer,
    presenter_port: Arc<PixelsPresenterPort>,
    width: u32,
    height: u32,
    has_frame: bool,
    last_error_message: Option<String>,
    last_render_duration: Option<Duration>,
}

impl PixelsPresenter {
    pub fn new(window: &'static Window, event_loop_proxy: EventLoopProxy<GuiEvent>) -> Self {
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
            presenter_port: Arc::new(PixelsPresenterPort {
                render_event: Mutex::new(None),
                event_loop_proxy,
            }),
            width: size.width,
            height: size.height,
            has_frame: false,
            last_error_message: None,
            last_render_duration: None,
        }
    }

    pub fn share_presenter_port(&self) -> Arc<dyn PresenterPort> {
        Arc::clone(&self.presenter_port) as Arc<dyn PresenterPort>
    }

    #[must_use]
    pub fn take_render_event(&self) -> Option<RenderEvent> {
        self.presenter_port.render_event.lock().unwrap().take()
    }

    fn draw_placeholder(&mut self) {
        let frame = self.pixels.frame_mut();
        for pixel in frame.chunks_exact_mut(4) {
            pixel[0] = 0;
            pixel[1] = 0;
            pixel[2] = 0;
            pixel[3] = 255;
        }
    }

    pub fn render(&mut self, egui_output: egui::FullOutput, egui_ctx: &EguiContext, requested_generation: u64) -> Result<(), pixels::Error> {
        if self.width == 0 || self.height == 0 {
            return Ok(());
        }

        let mut drew_frame = false;
        if let Some(event) = self.take_render_event() {
            match event {
                RenderEvent::Frame(frame) => {
                    let pixel_rect = frame.pixel_buffer.pixel_rect();
                    if frame.generation == requested_generation
                        && pixel_rect.width() == self.width
                        && pixel_rect.height() == self.height
                    {
                        self.copy_pixel_buffer_into_pixels_frame(&frame);
                        self.has_frame = true;
                        self.last_render_duration = Some(frame.render_duration);
                        self.last_error_message = None;
                        drew_frame = true;
                    }
                }
                RenderEvent::Error(error) => {
                    if error.generation == requested_generation {
                        self.last_error_message = Some(error.message);
                    }
                }
            }
        }

        if !drew_frame && !self.has_frame {
            self.draw_placeholder();
        }

        let clipped_primitives = egui_ctx.tessellate(egui_output.shapes, egui_ctx.pixels_per_point());

        let screen_descriptor = egui_wgpu::ScreenDescriptor {
            size_in_pixels: [self.width, self.height],
            pixels_per_point: egui_ctx.pixels_per_point(),
        };

        let textures_delta = egui_output.textures_delta;

        self.pixels.render_with(|encoder, render_target, context| {
            // First, render the pixels framebuffer (the scaling pass)
            context.scaling_renderer.render(encoder, render_target);

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

    pub fn resize(&mut self, width: u32, height: u32) {
        self.width = width;
        self.height = height;

        self.pixels
            .resize_surface(width, height)
            .expect("Failed to resize surface");
    }

    pub fn resize_pixels_buffer(&mut self, width: u32, height: u32) {
        self.width = width;
        self.height = height;

        self.pixels
            .resize_buffer(width, height)
            .expect("Failed to resize buffer");

        self.has_frame = false;
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

        copy_rgb_to_rgba(src, dest);
    }
}

impl PresenterPort for PixelsPresenterPort {
    fn present(&self, event: RenderEvent) {
        *self.render_event.lock().unwrap() = Some(event);
        let _ = self.event_loop_proxy.send_event(GuiEvent::Wake);
    }
}

#[path = "../framework.rs"]
mod framework;

use std::iter;

struct BindGroups {
    bind_group_render_result: wgpu::BindGroup,
}

struct Example {
    pipeline_render_result: wgpu::RenderPipeline,
    bind_group_layout_render_result: wgpu::BindGroupLayout,
    textures: BindGroups,
}

impl BindGroups {
    fn new(
        device: &wgpu::Device,
        width: u32,
        height: u32,
        bind_group_layout_render_result: &wgpu::BindGroupLayout,
    ) -> Self {
        let velocity_field = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("Velocity Field"),
            size: wgpu::Extent3d {
                width,
                height,
                depth: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rg32Float,
            usage: wgpu::TextureUsage::STORAGE | wgpu::TextureUsage::SAMPLED,
        });
        let velocity_field_view = velocity_field.create_view(&Default::default());

        let bind_group_render_result = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("BindGroup render result"),
            layout: bind_group_layout_render_result,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::TextureView(&velocity_field_view),
            }],
        });

        BindGroups {
            bind_group_render_result,
        }
    }
}

impl framework::Example for Example {
    fn init(
        sc_desc: &wgpu::SwapChainDescriptor,
        device: &wgpu::Device,
        _queue: &wgpu::Queue,
    ) -> Self {
        let bind_group_layout_render_result =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("BindGroupLayout render result"),
                entries: &[wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStage::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: false },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                }],
            });

        let pipeline_render_result = {
            let vs_module = device.create_shader_module(&wgpu::include_spirv!("shader.vert.spv"));
            let fs_module = device.create_shader_module(&wgpu::include_spirv!("shader.frag.spv"));

            let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: None,
                bind_group_layouts: &[&bind_group_layout_render_result],
                push_constant_ranges: &[],
            });

            device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some("Pipeline render result"),
                layout: Some(&pipeline_layout),
                vertex_stage: wgpu::ProgrammableStageDescriptor {
                    module: &vs_module,
                    entry_point: "main",
                },
                fragment_stage: Some(wgpu::ProgrammableStageDescriptor {
                    module: &fs_module,
                    entry_point: "main",
                }),
                rasterization_state: Some(wgpu::RasterizationStateDescriptor {
                    cull_mode: wgpu::CullMode::None,
                    ..Default::default()
                }),
                primitive_topology: wgpu::PrimitiveTopology::TriangleList,
                color_states: &[sc_desc.format.into()],
                depth_stencil_state: None,
                vertex_state: wgpu::VertexStateDescriptor {
                    index_format: None,
                    vertex_buffers: &[],
                },
                sample_count: 1,
                sample_mask: !0,
                alpha_to_coverage_enabled: false,
            })
        };

        let textures = BindGroups::new(
            device,
            sc_desc.width,
            sc_desc.height,
            &bind_group_layout_render_result,
        );

        Example {
            pipeline_render_result,
            bind_group_layout_render_result,
            textures,
        }
    }

    fn update(&mut self, event: winit::event::WindowEvent) {
        match event {
            winit::event::WindowEvent::KeyboardInput { input, .. } => {
                if let winit::event::ElementState::Pressed = input.state {
                    match input.virtual_keycode {
                        // TODO: Some interaction
                        Some(winit::event::VirtualKeyCode::Left) => {}
                        Some(winit::event::VirtualKeyCode::Right) => {}
                        _ => {}
                    }
                }
            }
            _ => {}
        }
    }

    fn resize(
        &mut self,
        sc_desc: &wgpu::SwapChainDescriptor,
        device: &wgpu::Device,
        _queue: &wgpu::Queue,
    ) {
        self.textures = BindGroups::new(
            device,
            sc_desc.width,
            sc_desc.height,
            &self.bind_group_layout_render_result,
        );
    }

    fn render(
        &mut self,
        frame: &wgpu::SwapChainTexture,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        _spawner: &impl futures::task::LocalSpawn,
    ) {
        let mut encoder =
            device.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });
        {
            let mut rpass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("render result to screen via screen filling triangle"),
                color_attachments: &[wgpu::RenderPassColorAttachmentDescriptor {
                    attachment: &frame.view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                        store: true,
                    },
                }],
                depth_stencil_attachment: None,
            });

            rpass.set_pipeline(&self.pipeline_render_result);
            rpass.set_bind_group(0, &self.textures.bind_group_render_result, &[]);
            rpass.draw(0..3, 0..1);
        }

        queue.submit(iter::once(encoder.finish()));
    }
}

fn main() {
    framework::run::<Example>("fluid-2d");
}

#[path = "../framework.rs"]
mod framework;

use std::iter;

struct BindGroups {
    advect: wgpu::BindGroup,
    compute_divergence: wgpu::BindGroup,
    solve_pressure: [wgpu::BindGroup; 2],
    remove_divergence: wgpu::BindGroup,
    render_result: wgpu::BindGroup,
}

struct Example {
    pipeline_render_result: wgpu::RenderPipeline,
    bind_group_layout_write_velocity: wgpu::BindGroupLayout,
    bind_group_layout_write_scalar: wgpu::BindGroupLayout,
    bind_group_layout_render_result: wgpu::BindGroupLayout,
    bind_groups: BindGroups,
}

const VELOCITY_TEXTURE_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rg32Float;
const SCALAR_TEXTURE_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::R32Float;

impl BindGroups {
    fn create_bind_group_write_velocity(
        label: &'static str,
        device: &wgpu::Device,
        bind_group_layout_write_velocity: &wgpu::BindGroupLayout,
        source_2d: &wgpu::TextureView,
        source_scalar: &wgpu::TextureView,
        dest_2d: &wgpu::TextureView,
    ) -> wgpu::BindGroup {
        device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some(label),
            layout: bind_group_layout_write_velocity,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&source_2d),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::TextureView(&source_scalar),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::TextureView(&dest_2d),
                },
            ],
        })
    }

    fn create_bind_group_write_scalar(
        label: &'static str,
        device: &wgpu::Device,
        bind_group_layout_write_scalar: &wgpu::BindGroupLayout,
        source_2d: &wgpu::TextureView,
        source_scalar0: &wgpu::TextureView,
        source_scalar1: &wgpu::TextureView,
        dest_scalar: &wgpu::TextureView,
    ) -> wgpu::BindGroup {
        device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some(label),
            layout: bind_group_layout_write_scalar,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&source_2d),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::TextureView(&source_scalar0),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::TextureView(&source_scalar1),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: wgpu::BindingResource::TextureView(&dest_scalar),
                },
            ],
        })
    }

    fn new(
        device: &wgpu::Device,
        width: u32,
        height: u32,
        bind_group_layout_write_velocity: &wgpu::BindGroupLayout,
        bind_group_layout_write_scalar: &wgpu::BindGroupLayout,
        bind_group_layout_render_result: &wgpu::BindGroupLayout,
    ) -> Self {
        let create_storage_texture = |label, format| {
            device
                .create_texture(&wgpu::TextureDescriptor {
                    label: Some(label),
                    size: wgpu::Extent3d {
                        width,
                        height,
                        depth: 1,
                    },
                    mip_level_count: 1,
                    sample_count: 1,
                    dimension: wgpu::TextureDimension::D2,
                    format,
                    usage: wgpu::TextureUsage::STORAGE,
                })
                .create_view(&Default::default())
        };

        let velocity_view = create_storage_texture("Velocity", VELOCITY_TEXTURE_FORMAT);
        let intermediary_velocity_view =
            create_storage_texture("Intermediary Velocity", VELOCITY_TEXTURE_FORMAT);
        let divergence_view = create_storage_texture("Divergence", SCALAR_TEXTURE_FORMAT);
        let pressure0_view = create_storage_texture("Pressure 0", SCALAR_TEXTURE_FORMAT);
        let pressure1_view = create_storage_texture("Pressure 1", SCALAR_TEXTURE_FORMAT);

        let advect = Self::create_bind_group_write_velocity(
            "BindGroup advect",
            device,
            bind_group_layout_write_velocity,
            &velocity_view,
            &divergence_view, // Dummy, not used
            &intermediary_velocity_view,
        );
        let compute_divergence = Self::create_bind_group_write_scalar(
            "BindGroup advect",
            device,
            bind_group_layout_write_scalar,
            &intermediary_velocity_view,
            &pressure0_view, // Dummy, unused
            &pressure1_view, // Dummy, unused
            &divergence_view,
        );
        let solve_pressure = [
            Self::create_bind_group_write_scalar(
                "BindGroup pressure solve p0->p1",
                device,
                bind_group_layout_write_scalar,
                &intermediary_velocity_view, // Dummy, unused
                &divergence_view,
                &pressure0_view,
                &pressure1_view,
            ),
            Self::create_bind_group_write_scalar(
                "BindGroup pressure solve p1->p0",
                device,
                bind_group_layout_write_scalar,
                &intermediary_velocity_view, // Dummy, unused
                &divergence_view,
                &pressure1_view,
                &pressure0_view,
            ),
        ];
        let remove_divergence = Self::create_bind_group_write_scalar(
            "BindGroup advect",
            device,
            bind_group_layout_write_scalar,
            &intermediary_velocity_view,
            &pressure0_view, // Dummy, unused
            &pressure1_view, // Dummy, unused
            &divergence_view,
        );
        let remove_divergence = Self::create_bind_group_write_velocity(
            "BindGroup remove divergence",
            device,
            bind_group_layout_write_velocity,
            &intermediary_velocity_view,
            &pressure0_view,
            &velocity_view,
        );

        let render_result = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("BindGroup render result"),
            layout: bind_group_layout_render_result,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::TextureView(&velocity_view),
            }],
        });

        BindGroups {
            advect,
            compute_divergence,
            solve_pressure,
            remove_divergence,
            render_result,
        }
    }
}

impl framework::Example for Example {
    fn init(
        sc_desc: &wgpu::SwapChainDescriptor,
        device: &wgpu::Device,
        _queue: &wgpu::Queue,
    ) -> Self {
        let binding_read_velocity = wgpu::BindingType::StorageTexture {
            access: wgpu::StorageTextureAccess::ReadOnly,
            format: VELOCITY_TEXTURE_FORMAT,
            view_dimension: wgpu::TextureViewDimension::D2,
        };
        let binding_write_velocity = wgpu::BindingType::StorageTexture {
            access: wgpu::StorageTextureAccess::WriteOnly,
            format: VELOCITY_TEXTURE_FORMAT,
            view_dimension: wgpu::TextureViewDimension::D2,
        };
        let binding_read_scalar = wgpu::BindingType::StorageTexture {
            access: wgpu::StorageTextureAccess::ReadOnly,
            format: SCALAR_TEXTURE_FORMAT,
            view_dimension: wgpu::TextureViewDimension::D2,
        };
        let binding_write_scalar = wgpu::BindingType::StorageTexture {
            access: wgpu::StorageTextureAccess::WriteOnly,
            format: SCALAR_TEXTURE_FORMAT,
            view_dimension: wgpu::TextureViewDimension::D2,
        };

        // Bind group for reading scalar and a velocity texture and writing another one.
        let bind_group_layout_write_velocity =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("BindGroupLayout write velocity"),
                entries: &[
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStage::COMPUTE,
                        ty: binding_read_velocity,
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: wgpu::ShaderStage::COMPUTE,
                        ty: binding_read_scalar,
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 2,
                        visibility: wgpu::ShaderStage::COMPUTE,
                        ty: binding_write_velocity,
                        count: None,
                    },
                ],
            });
        // Bind group for reading two scalar textures writing to a scalar texture.
        let bind_group_layout_write_scalar =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("BindGroupLayout compute divergence"),
                entries: &[
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStage::COMPUTE,
                        ty: binding_read_velocity,
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: wgpu::ShaderStage::COMPUTE,
                        ty: binding_read_scalar,
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 2,
                        visibility: wgpu::ShaderStage::COMPUTE,
                        ty: binding_read_scalar,
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 3,
                        visibility: wgpu::ShaderStage::COMPUTE,
                        ty: binding_write_scalar,
                        count: None,
                    },
                ],
            });
        // Bind group for rendering the results - reads velocity texture.
        let bind_group_layout_render_result =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("BindGroupLayout render result"),
                entries: &[wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStage::FRAGMENT,
                    ty: binding_read_velocity,
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

        let bind_groups = BindGroups::new(
            device,
            sc_desc.width,
            sc_desc.height,
            &bind_group_layout_write_velocity,
            &bind_group_layout_write_scalar,
            &bind_group_layout_render_result,
        );

        Example {
            pipeline_render_result,
            bind_group_layout_write_velocity,
            bind_group_layout_write_scalar,
            bind_group_layout_render_result,
            bind_groups,
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
        self.bind_groups = BindGroups::new(
            device,
            sc_desc.width,
            sc_desc.height,
            &self.bind_group_layout_write_velocity,
            &self.bind_group_layout_write_scalar,
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
            rpass.set_bind_group(0, &self.bind_groups.render_result, &[]);
            rpass.draw(0..3, 0..1);
        }

        queue.submit(iter::once(encoder.finish()));
    }
}

fn main() {
    framework::run::<Example>("fluid-2d");
}

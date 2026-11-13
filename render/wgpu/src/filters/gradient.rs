use crate::backend::RenderTargetMode;
use crate::buffer_pool::TexturePool;
use crate::descriptors::Descriptors;
use crate::filters::blur::BlurFilter;
use crate::filters::{FilterSource, FilterVertex, VERTEX_BUFFERS_DESCRIPTION_FILTERS};
use crate::mesh::{GRADIENT_SIZE, gradient_colors};
use crate::surface::target::CommandTarget;
use crate::utils::SampleCountMap;
use bytemuck::{Pod, Zeroable};
use std::sync::OnceLock;
use swf::{GradientFilter as GradientFilterArgs, GradientInterpolation, Rectangle, Twips};
use wgpu::util::StagingBelt;

#[repr(C)]
#[derive(Copy, Clone, Debug, Pod, Zeroable, PartialEq)]
struct GradientFilterUniform {
    colors: [[f32; 4]; GRADIENT_SIZE],
    area: [f32; 4],
    blur: [f32; 4],
    strength: f32,
    filter_type: u32, // 0 outer, 1 inner, 2 full
    knockout: u32,    // a wasteful bool, but we need to be aligned anyway
    bevel: u32,       // 0 gradient glow, 1 gradient bevel
}

/// Which filter a `GradientFilter` applies.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum GradientFilterKind {
    Glow,
    Bevel,
}

/// Gradient glow and gradient bevel: a glow or bevel whose blurred alpha picks
/// a color from a gradient.
pub struct GradientFilter {
    bind_group_layout: wgpu::BindGroupLayout,
    pipeline_layout: wgpu::PipelineLayout,
    vertex_buffer: wgpu::Buffer,
    uniform_buffer: wgpu::Buffer,
    vertices_size: wgpu::BufferSize,
    uniform_size: wgpu::BufferSize,
    pipeline: SampleCountMap<OnceLock<wgpu::RenderPipeline>>,
}

impl GradientFilter {
    pub fn new(device: &wgpu::Device) -> Self {
        let uniform_size = std::mem::size_of::<GradientFilterUniform>() as u64;
        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        multisampled: false,
                        sample_type: wgpu::TextureSampleType::Float { filterable: false },
                        view_dimension: wgpu::TextureViewDimension::D2,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::NonFiltering),
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: wgpu::BufferSize::new(uniform_size),
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 3,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        multisampled: false,
                        sample_type: wgpu::TextureSampleType::Float { filterable: false },
                        view_dimension: wgpu::TextureViewDimension::D2,
                    },
                    count: None,
                },
            ],
            label: create_debug_label!("Gradient filter binds").as_deref(),
        });

        let vertices_size = std::mem::size_of::<[FilterVertex; 4]>() as u64;
        let vertex_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: None,
            size: vertices_size,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let uniform_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: None,
            size: uniform_size,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: None,
            bind_group_layouts: &[Some(&bind_group_layout)],
            immediate_size: 0,
        });

        Self {
            pipeline: Default::default(),
            pipeline_layout,
            vertex_buffer,
            uniform_buffer,
            bind_group_layout,
            uniform_size: wgpu::BufferSize::new(uniform_size).expect("Definitely not zero."),
            vertices_size: wgpu::BufferSize::new(vertices_size).expect("Definitely not zero."),
        }
    }

    fn pipeline(&self, descriptors: &Descriptors, msaa_sample_count: u32) -> &wgpu::RenderPipeline {
        self.pipeline.get_or_init(msaa_sample_count, || {
            let label = create_debug_label!("Gradient Filter ({} msaa)", msaa_sample_count);
            descriptors
                .device
                .create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                    label: label.as_deref(),
                    layout: Some(&self.pipeline_layout),
                    vertex: wgpu::VertexState {
                        module: &descriptors.shaders.gradient_filter,
                        entry_point: Some("main_vertex"),
                        buffers: &VERTEX_BUFFERS_DESCRIPTION_FILTERS,
                        compilation_options: Default::default(),
                    },
                    primitive: wgpu::PrimitiveState {
                        topology: wgpu::PrimitiveTopology::TriangleList,
                        strip_index_format: None,
                        front_face: wgpu::FrontFace::Ccw,
                        cull_mode: None,
                        polygon_mode: wgpu::PolygonMode::default(),
                        unclipped_depth: false,
                        conservative: false,
                    },
                    depth_stencil: None,
                    multisample: wgpu::MultisampleState {
                        count: msaa_sample_count,
                        mask: !0,
                        alpha_to_coverage_enabled: false,
                    },
                    fragment: Some(wgpu::FragmentState {
                        module: &descriptors.shaders.gradient_filter,
                        entry_point: Some("main_fragment"),
                        targets: &[Some(wgpu::TextureFormat::Rgba8Unorm.into())],
                        compilation_options: Default::default(),
                    }),
                    multiview_mask: None,
                    cache: None,
                })
        })
    }

    #[expect(clippy::too_many_arguments)]
    pub fn apply(
        &self,
        descriptors: &Descriptors,
        texture_pool: &mut TexturePool,
        draw_encoder: &mut wgpu::CommandEncoder,
        staging_belt: &mut StagingBelt,
        source: &FilterSource,
        filter: &GradientFilterArgs,
        kind: GradientFilterKind,
        area: Option<Rectangle<Twips>>,
        blur_filter: &BlurFilter,
    ) -> CommandTarget {
        let sample_count = source.texture.sample_count();
        let format = source.texture.format();
        let pipeline = self.pipeline(descriptors, sample_count);
        let blurred = blur_filter.apply(
            descriptors,
            texture_pool,
            draw_encoder,
            staging_belt,
            source,
            &filter.inner_blur_filter(),
        );
        let blurred_texture = if let Some(blurred) = &blurred {
            blurred.ensure_cleared(draw_encoder);
            blurred.color_texture()
        } else {
            source.texture
        };
        let source_view = source.texture.create_view(&Default::default());
        let blurred_view = blurred_texture.create_view(&Default::default());
        let distance = filter.distance.to_f32();
        let angle = filter.angle.to_f32();
        // The angle is stored in fixed point, so e.g. 180 degrees give an
        // offset just short of whole pixels, which the rounding down of the
        // interpolated alpha would notice.
        let snap = |offset: f32| (offset * 256.0).round() / 256.0;
        let blur_offset = (snap(angle.cos() * distance), snap(angle.sin() * distance));

        let target = CommandTarget::new(
            descriptors,
            texture_pool,
            wgpu::Extent3d {
                width: source.size.0,
                height: source.size.1,
                depth_or_array_layers: 1,
            },
            format,
            sample_count,
            RenderTargetMode::FreshWithColor(wgpu::Color::TRANSPARENT),
            draw_encoder,
        );
        // The blurred texture only holds the source's area; without a blur,
        // the source texture is sampled instead.
        let blur_origin = if blurred.is_some() {
            (0.0, 0.0)
        } else {
            (source.point.0 as f32, source.point.1 as f32)
        };
        let gradient = gradient_colors(&filter.colors, GradientInterpolation::Rgb);
        let mut colors = [[0.0; 4]; GRADIENT_SIZE];
        for (color, rgba) in colors.iter_mut().zip(gradient.as_chunks::<4>().0) {
            *color = rgba.map(|value| f32::from(value) / 255.0);
        }
        staging_belt
            .write_buffer(draw_encoder, &self.uniform_buffer, 0, self.uniform_size)
            .copy_from_slice(bytemuck::cast_slice(&[GradientFilterUniform {
                colors,
                // Without an area, the filter draws to the whole texture.
                area: area.map_or([f32::MIN, f32::MIN, f32::MAX, f32::MAX], |area| {
                    [
                        area.x_min.to_pixels() as f32,
                        area.y_min.to_pixels() as f32,
                        area.x_max.to_pixels() as f32,
                        area.y_max.to_pixels() as f32,
                    ]
                }),
                blur: [blur_origin.0, blur_origin.1, blur_offset.0, blur_offset.1],
                strength: filter.strength.to_f32(),
                filter_type: if filter.is_on_top() {
                    2
                } else if filter.is_inner() {
                    1
                } else {
                    0
                },
                knockout: if filter.is_knockout() { 1 } else { 0 },
                bevel: if kind == GradientFilterKind::Bevel {
                    1
                } else {
                    0
                },
            }]));
        staging_belt
            .write_buffer(draw_encoder, &self.vertex_buffer, 0, self.vertices_size)
            .copy_from_slice(bytemuck::cast_slice(&[source.vertices()]));
        let filter_group = descriptors
            .device
            .create_bind_group(&wgpu::BindGroupDescriptor {
                label: create_debug_label!("Filter group").as_deref(),
                layout: &self.bind_group_layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: wgpu::BindingResource::TextureView(&source_view),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: wgpu::BindingResource::Sampler(
                            descriptors.bitmap_samplers.get_sampler(false, false),
                        ),
                    },
                    wgpu::BindGroupEntry {
                        binding: 2,
                        resource: self.uniform_buffer.as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 3,
                        resource: wgpu::BindingResource::TextureView(&blurred_view),
                    },
                ],
            });
        let mut render_pass = draw_encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: create_debug_label!("Gradient filter").as_deref(),
            color_attachments: &[target.color_attachments()],
            ..Default::default()
        });
        render_pass.set_pipeline(pipeline);

        render_pass.set_bind_group(0, &filter_group, &[]);

        render_pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
        render_pass.set_index_buffer(
            descriptors.quad.index_buffer.slice(..),
            wgpu::IndexFormat::Uint32,
        );
        render_pass.draw_indexed(descriptors.quad.indices_quad.clone(), 0, 0..1);
        target
    }
}

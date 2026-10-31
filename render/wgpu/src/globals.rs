//use super::utils::create_debug_label;
use bytemuck::{Pod, Zeroable};
use ruffle_render::matrix3d::Matrix3D;
use wgpu::util::DeviceExt;

#[derive(Debug)]
pub struct Globals {
    bind_group: wgpu::BindGroup,
    _buffer: wgpu::Buffer,
}

#[repr(C)]
#[derive(Copy, Clone, Debug, Pod, Zeroable)]
pub struct GlobalsUniform {
    global_matrix: [[f32; 4]; 4],
}

impl Globals {
    pub fn new(
        device: &wgpu::Device,
        layout: &wgpu::BindGroupLayout,
        viewport_width: u32,
        viewport_height: u32,
    ) -> Self {
        // Maps viewport pixels to clip space. Perspective projections are part
        // of each draw's own matrix (see `TransformStack`), which leaves z at 0.
        let vw = viewport_width as f32 / 2.0;
        let vh = viewport_height as f32 / 2.0;
        let global_matrix = Matrix3D {
            #[rustfmt::skip]
            raw_data: [
                (1.0 / vw), 0.0, 0.0, 0.0,
                0.0, (-1.0 / vh), 0.0, 0.0,
                0.0, 0.0, 0.0, 0.0,
                -1.0, 1.0, 0.0, 1.0,
            ],
        };

        let global_matrix: [[f32; 4]; 4] = bytemuck::cast(global_matrix.raw_data);

        let temp_label = create_debug_label!("Globals buffer");
        let buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: temp_label.as_deref(),
            contents: bytemuck::cast_slice(&[GlobalsUniform { global_matrix }]),
            usage: wgpu::BufferUsages::UNIFORM,
        });

        let bind_group_label = create_debug_label!("Globals bind group");
        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: bind_group_label.as_deref(),
            layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: buffer.as_entire_binding(),
            }],
        });

        Self {
            bind_group,
            _buffer: buffer,
        }
    }

    pub fn bind_group(&self) -> &wgpu::BindGroup {
        &self.bind_group
    }
}

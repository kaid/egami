#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub(crate) struct Vertex {
    pub(crate) position: [f32; 2],
    pub(crate) texture_coords: [f32; 2],
}

impl Vertex {
    pub(crate) const ATTRIBBS: [wgpu::VertexAttribute; 2] = wgpu::vertex_attr_array![0 => Float32x2, 1 => Float32x2];

    pub(crate) fn desc() -> wgpu::VertexBufferLayout<'static> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<Vertex>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &Self::ATTRIBBS,
        }
    }
}

pub(crate) const VERTICES: &[Vertex] = &[
    Vertex { position: [-1.0, 1.0], texture_coords: [0.0, 0.0], }, // 0
    Vertex { position: [1.0, 1.0], texture_coords: [1.0, 0.0], }, // 1
    Vertex { position: [-1.0, -1.0], texture_coords: [0.0, 1.0], }, // 2
    Vertex { position: [1.0, -1.0], texture_coords: [1.0, 1.0], }, // 3
];

pub(crate) const INDICES: &[usize] = &[
    0, 2, 1,
    2, 3, 1,
];
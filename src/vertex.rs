use crate::viewport::ViewPortMargin;

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub(crate) struct Vertex {
    position: [f32; 2],
    texture_coords: [f32; 2],
}

impl Vertex {
    pub(crate) const ATTRIBS: [wgpu::VertexAttribute; 2] = wgpu::vertex_attr_array![0 => Float32x2, 1 => Float32x2];

    pub(crate) fn desc() -> wgpu::VertexBufferLayout<'static> {
        wgpu::VertexBufferLayout {
            attributes: &Self::ATTRIBS,
            step_mode: wgpu::VertexStepMode::Vertex,
            array_stride: std::mem::size_of::<Vertex>() as wgpu::BufferAddress,
        }
    }

    pub(crate) fn get_vertices(aspect_ratios: (f32, f32)) -> [Self; 4] {
        let (h_margin, v_margin) = ViewPortMargin::from(aspect_ratios).into();

        [
            Self { position: [-1.0 + h_margin, 1.0 - v_margin], texture_coords: [0.0, 0.0] },
            Self { position: [1.0 - h_margin, 1.0 - v_margin], texture_coords: [1.0, 0.0] },
            Self { position: [-1.0 + h_margin, -1.0 + v_margin], texture_coords: [0.0, 1.0] },
            Self { position: [1.0 - h_margin, -1.0 + v_margin], texture_coords: [1.0, 1.0] },
        ]
    }
}

pub(crate) const INDICES: &[u16] = &[
    0, 2, 1,
    2, 3, 1,
];
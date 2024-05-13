#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub(crate) struct Vertex {
    position: [f32; 2],
    texture_coords: [f32; 2],
}

enum ViewPortMargin {
    Horizontal(f32),
    Vertical(f32),
}

impl ViewPortMargin {
    // image_aspect_ratio = image_h / image_w
    // viewport_aspect_ratio = viewport_h / viewport_w
    // vertex coords are in range [-1, 1], origin at the center
    // we want to place the image in the middle of the viewport
    // check whether the image is wider than the viewport
    // if so, we have a horizontal margin
    // if not, we have a vertical margin
    pub fn from<T: Into<(f32, f32)>>(aspect_ratios: T) -> Self {
        let (object_aspect_ratio, viewport_aspect_ratio) = aspect_ratios.into();

        if object_aspect_ratio > viewport_aspect_ratio {
            ViewPortMargin::Horizontal((1.0 - viewport_aspect_ratio / object_aspect_ratio) / 2.0)
        } else {
            ViewPortMargin::Vertical((1.0 - object_aspect_ratio / viewport_aspect_ratio) / 2.0)
        }
    }
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

    pub(crate) fn from(image_aspect_ratio: f32, viewport_aspect_ratio: f32) -> [Self; 4] {
        let (h_margin, v_margin) = match ViewPortMargin::from((image_aspect_ratio, viewport_aspect_ratio)) {
            ViewPortMargin::Horizontal(margin) => (margin, 0.0),
            ViewPortMargin::Vertical(margin) => (0.0, margin),
        };

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
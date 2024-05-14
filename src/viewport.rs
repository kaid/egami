pub(crate) enum ViewPortMargin {
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
            ViewPortMargin::Horizontal(1.0 - viewport_aspect_ratio / object_aspect_ratio)
        } else {
            ViewPortMargin::Vertical(1.0 - object_aspect_ratio / viewport_aspect_ratio)
        }
    }
}

// (horizontal margin, vertical margin)
impl Into<(f32, f32)> for ViewPortMargin {
    fn into(self) -> (f32, f32) {
        match self {
            ViewPortMargin::Horizontal(margin) => (margin, 0.0),
            ViewPortMargin::Vertical(margin) => (0.0, margin),
        }
    }
}
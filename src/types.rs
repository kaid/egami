pub type Pair<Type> = (Type, Type);

pub trait HasSize<Type> {
    fn size(&self) -> Pair<Type>;
}

pub trait HasPosition<Type> {
    fn position(&self) -> Pair<Type>;
}

pub trait HasRatio {
    fn ratio(&self) -> f32;
    fn inverse_ratio(&self) -> f32;
}

pub trait HasData {
    fn data(&self) -> &[u8];
}

pub trait FrameRenderContext: From<Self::Init> + HasSize<u32> {
    type Init;
    type RenderError;

    fn init(init: Self::Init) -> Self {
        let mut instance = Self::from(init);
        let size = instance.size();
        instance.configure(size);
        instance
    }

    fn configure(&mut self, size: Pair<u32>);

    fn draw_frame<Frame>(&mut self, frame_provider: impl Iterator<Item = Frame>) -> Result<(), Self::RenderError>
    where
        Frame: HasSize<u32> + HasPosition<u32> + HasData;
}

use super::ModUi;
use crate::render::renderable::FlexRenderable;
use crate::render::renderable::RenderableItem;

impl ModUi {
    pub(crate) fn above_composer<'a>(&'a self, composer: RenderableItem<'a>) -> RenderableItem<'a> {
        let mut layout = FlexRenderable::new();
        let mod_flex = 1;
        layout.push(mod_flex, RenderableItem::Borrowed(self));
        let composer_flex = 0;
        layout.push(composer_flex, composer);
        RenderableItem::Owned(Box::new(layout))
    }
}

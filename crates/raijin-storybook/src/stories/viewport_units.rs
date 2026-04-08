use inazuma::Render;
use inazuma_story::Story;

use raijin_ui::prelude::*;

pub struct ViewportUnitsStory;

impl Render for ViewportUnitsStory {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        Story::container(cx).child(
            div()
                .flex()
                .flex_row()
                .child(
                    div()
                        .w(vw(0.5, window))
                        .h(vh(0.8, window))
                        .bg(inazuma::red())
                        .text_color(inazuma::white())
                        .child("50vw, 80vh"),
                )
                .child(
                    div()
                        .w(vw(0.25, window))
                        .h(vh(0.33, window))
                        .bg(inazuma::green())
                        .text_color(inazuma::white())
                        .child("25vw, 33vh"),
                ),
        )
    }
}

use std::error::Error;

use graphics::mesh::ShapeStyle;
use tetra::{
    graphics::{self, mesh::Mesh, Color, Rectangle, StencilAction, StencilFunction},
    math::Vec2,
    Context, ContextBuilder, State,
};

struct MainState;

impl State<Box<dyn Error>> for MainState {
    fn draw(&mut self, ctx: &mut Context) -> Result<(), Box<dyn Error>> {
        graphics::clear(ctx, Color::BLACK);
        graphics::stencil(
            ctx,
            StencilAction::Replace(1),
            |ctx| -> Result<(), Box<dyn Error>> {
                Mesh::circle(ctx, ShapeStyle::Fill, Vec2::new(400.0, 300.0), 200.0)?
                    .draw(ctx, Vec2::zero());
                Ok(())
            },
        )?;
        graphics::set_stencil_test(ctx, StencilFunction::EqualTo(1));
        Mesh::rectangle(
            ctx,
            ShapeStyle::Fill,
            Rectangle::new(400.0, 300.0, 500.0, 500.0),
        )?
        .draw(ctx, Vec2::zero());
        graphics::set_stencil_test(ctx, Default::default());

        Ok(())
    }
}

fn main() -> Result<(), Box<dyn Error>> {
    ContextBuilder::new("test", 800, 600)
        .build()?
        .run(|_| Ok(MainState))
}

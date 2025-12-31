use crate::core::data::pixel_rect::PixelRect;
use crate::core::data::point::Point;
use crate::core::actions::generate_fractal::ports::fractal_algorithm::FractalAlgorithm;

pub fn generate_fractal<Alg: FractalAlgorithm>(pixel_rect: PixelRect, algorithm: &Alg) -> Result<Vec<Alg::Success>, Alg::Failure>
{
    (pixel_rect.top_left().y..pixel_rect.bottom_right().y)
        .flat_map(|y| {
            (pixel_rect.top_left().x..pixel_rect.bottom_right().x)
                .map(move |x| Point { x, y })
        })
        .map(|pixel| {
            let result = algorithm.compute(pixel)?;
            Ok(result)
        })
        .collect()
}

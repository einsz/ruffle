use crate::commands::CommandHandler;
use crate::matrix::Matrix;
use crate::matrix3d::Matrix3D;
use crate::perspective_projection::PerspectiveProjection;
use swf::{Color, ColorTransform};

/// Represents the transform for a DisplayObject.
/// This includes both the transformation matrix and the color transform.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Transform {
    /// The 2D matrix. For a 3D transform, this is its projection onto the XY plane.
    pub matrix: Matrix,
    pub color_transform: ColorTransform,
    pub perspective_projection: Option<PerspectiveProjection>,
    /// The 3D matrix, in pixels, if this transform is 3D.
    ///
    /// On a `TransformStack`, this is the complete matrix from the 3D object's
    /// space to the screen, including the perspective projection.
    pub matrix3d: Option<Matrix3D>,
}

impl Transform {
    /// Scales the object before this transform, e.g. to the size of a bitmap.
    pub fn prepend_scale(&mut self, x: f32, y: f32) {
        self.matrix *= Matrix::scale(x, y);
        if let Some(matrix3d) = &mut self.matrix3d {
            *matrix3d = matrix3d.multiply(&Matrix3D::scale(x, y, 1.0));
        }
    }

    /// Draws a rectangle, given as a matrix from the unit square into the
    /// object's space, with this transform.
    pub fn draw_rect(&self, commands: &mut impl CommandHandler, color: Color, rect: Matrix) {
        let matrix = self.matrix * rect;
        match self.matrix3d {
            Some(matrix3d) => commands.draw_rect_3d(
                color,
                matrix,
                matrix3d.multiply(&Matrix3D::from_matrix(rect)),
            ),
            None => commands.draw_rect(color, matrix),
        }
    }

    fn to_matrix3d(&self) -> Matrix3D {
        self.matrix3d
            .unwrap_or_else(|| Matrix3D::from_matrix(self.matrix))
    }
}

/// A perspective projection in effect, with the 2D matrix from its space to the screen.
#[derive(Clone, Copy, Debug)]
struct Projection {
    base: Matrix,
    perspective_projection: PerspectiveProjection,
}

impl Projection {
    /// Projects from 3D onto the XY plane of the projection's space, dropping z.
    fn to_matrix3d(self, stage_width: f32) -> Matrix3D {
        let focal_length = self.perspective_projection.focal_length(stage_width);
        let (center_x, center_y) = self.perspective_projection.center;
        // x' = center_x + (x - center_x) * focal_length / (focal_length + z),
        // and likewise for y.
        Matrix3D {
            #[rustfmt::skip]
            raw_data: [
                1.0, 0.0, 0.0, 0.0,
                0.0, 1.0, 0.0, 0.0,
                center_x as f32 / focal_length, center_y as f32 / focal_length, 0.0, 1.0 / focal_length,
                0.0, 0.0, 0.0, 1.0,
            ],
        }
    }
}

pub struct TransformStack {
    stack: Vec<(Transform, Option<Projection>)>,
    stage_width: f32,
}

impl TransformStack {
    pub fn new() -> Self {
        Self {
            stack: vec![(Transform::default(), None)],
            stage_width: 500.0,
        }
    }

    /// A stack for rendering the current object into an image, which
    /// `image_to_screen` places on the screen. 3D objects in it are projected as
    /// they are on the screen, unless the current object is 3D itself.
    pub fn offscreen(&self, image_to_screen: Matrix) -> Self {
        let (transform, projection) = &self.stack[self.stack.len() - 1];
        let projection = projection
            .filter(|_| transform.matrix3d.is_none())
            .and_then(|projection| {
                Some(Projection {
                    base: image_to_screen.inverse()? * projection.base,
                    ..projection
                })
            });
        Self {
            stack: vec![(Transform::default(), projection)],
            stage_width: self.stage_width,
        }
    }

    /// Projects 3D objects with the default field of view, centred on the
    /// render target and with its width as the stage width, as when drawing
    /// into a bitmap.
    pub fn set_target_projection(&mut self, width: u32, height: u32) {
        self.stack[0].1 = Some(Projection {
            base: Matrix::IDENTITY,
            perspective_projection: PerspectiveProjection {
                center: (width as f64 / 2.0, height as f64 / 2.0),
                ..Default::default()
            },
        });
        self.stage_width = width as f32;
    }

    /// Sets the stage width, which perspective projections' focal lengths depend on.
    pub fn set_stage_width(&mut self, stage_width: f32) {
        self.stage_width = stage_width;
    }

    pub fn push(&mut self, transform: &Transform) {
        let (cur_transform, cur_projection) = self.stack[self.stack.len() - 1].clone();
        let matrix = cur_transform.matrix * transform.matrix;
        let color_transform = cur_transform.color_transform * transform.color_transform;

        let matrix3d = if let Some(cur_matrix3d) = cur_transform.matrix3d {
            Some(cur_matrix3d.multiply(&transform.to_matrix3d()))
        } else if let Some(matrix3d) = transform.matrix3d {
            // The outermost 3D object: it is projected onto the plane of the
            // perspective projection's space, which the 2D matrices above
            // that then transform as usual.
            let (base, projection) = match cur_projection {
                Some(projection) => (projection.base, projection.to_matrix3d(self.stage_width)),
                None => (Matrix::IDENTITY, Matrix3D::scale(1.0, 1.0, 0.0)),
            };
            let base_to_parent = base.inverse().unwrap_or(Matrix::ZERO) * cur_transform.matrix;
            Some(
                Matrix3D::from_matrix(base)
                    .multiply(&projection)
                    .multiply(&Matrix3D::from_matrix(base_to_parent))
                    .multiply(&matrix3d),
            )
        } else {
            None
        };

        // A perspective projection applies to the children of its object.
        let projection = match transform.perspective_projection {
            Some(perspective_projection) if matrix3d.is_none() => Some(Projection {
                base: matrix,
                perspective_projection,
            }),
            _ => cur_projection,
        };

        self.stack.push((
            Transform {
                matrix,
                color_transform,
                perspective_projection: None,
                matrix3d,
            },
            projection,
        ));
    }

    pub fn pop(&mut self) {
        assert!(self.stack.len() > 1, "Transform stack underflow");
        self.stack.pop();
    }

    /// Whether the current transform is 3D.
    pub fn is_3d(&self) -> bool {
        self.stack[self.stack.len() - 1].0.matrix3d.is_some()
    }

    pub fn transform(&self) -> Transform {
        self.stack[self.stack.len() - 1].0.clone()
    }
}

impl Default for TransformStack {
    fn default() -> Self {
        TransformStack::new()
    }
}

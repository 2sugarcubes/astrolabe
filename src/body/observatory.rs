use coordinates::prelude::{Spherical, ThreeDimensionalConsts, Vector3};
use quaternion::Quaternion;

use crate::Float;

use super::ArcBody;

pub struct Observatory {
    location: Quaternion<Float>,
    body: ArcBody,
}

impl Observatory {
    pub fn new(location: Spherical<Float>, body: ArcBody) -> Self {
        let location: Vector3<Float> = location.into();
        Self {
            location: quaternion::rotation_from_to(location.into(), Vector3::UP.into()),
            body,
        }
    }

    /// Takes bodies from a universal coordinate space and converts them to local coordinates
    /// relative to the observatory
    pub fn observe(&self, time: Float) -> Vec<(ArcBody, Spherical<Float>)> {
        let body = self.body.read().unwrap();
        let raw_observations = body.get_observations_from_here(time);

        let rotation = if let Some(rotation) = &body.rotation {
            quaternion::mul(self.location, rotation.get_rotation(time))
        } else {
            self.location
        };

        // Rotate observations to put them in the local coordinate space
        raw_observations
            .iter()
            .filter_map(|(body, pos)| {
                let local_coordinates =
                    Vector3::from(quaternion::rotate_vector(rotation, (*pos).into()));

                // Filter out bodies below the horizon
                if local_coordinates.z >= 0.0 {
                    Some((body.clone(), local_coordinates.into()))
                } else {
                    None
                }
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    //FIXME write some tests later, my head hurts

    use coordinates::{
        prelude::{Spherical, ThreeDimensionalConsts, Vector3},
        traits::{Cross3D, Magnitude},
    };

    use crate::{
        body::{observatory::Observatory, rotating_body::RotatingBody, ArcBody, Body},
        consts::float,
        dynamic::fixed::Fixed,
        Float,
    };

    fn get_toy_example_body() -> ArcBody {
        let body = Body::new(None, Fixed::new(Vector3::ORIGIN));
        body.write().unwrap().rotation = Some(RotatingBody::new(4.0, Spherical::UP));
        let _ = Body::new(Some(body.clone()), Fixed::new(Vector3::RIGHT));
        //                        \-> ONCE TOLD ME. Now you can't get it out of your head either
        //let _ = Body::new(Some(body.clone()), Fixed::new(Vector3::BACK));
        //let _ = Body::new(Some(body.clone()), Fixed::new(Vector3::LEFT));
        //let _ = Body::new(Some(body.clone()), Fixed::new(Vector3::FORWARD));

        body
    }

    #[test]
    fn simple_rotation_test() {
        let root = get_toy_example_body();
        let observitory = Observatory::new(Spherical::RIGHT, root);

        for (time, polar_angle) in [
            (0_u8, 0.0),
            (1, float::FRAC_PI_2),
            (2, float::PI),
            (3, float::FRAC_PI_2),
        ] {
            let observations: Vec<Spherical<Float>> = observitory
                .observe(Float::from(time))
                .iter()
                .map(|(_, loc)| loc.clone())
                .collect();

            println!("{observations:.2?}");

            if observations.len() == 0 {
                assert!(time == 2);
            } else {
                assert_float_absolute_eq!(observations[0].polar_angle, polar_angle);
            }
        }
    }
}
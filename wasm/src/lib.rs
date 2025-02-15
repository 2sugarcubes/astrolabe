//! Facade layer to link from javascript to the celestial body simulation engine

use astrograph::{
    body::{
        observatory::{self, Observatory, WeakObservatory},
        rotating::Rotating,
    },
    dynamic::{fixed::Fixed, keplerian::Keplerian},
    generator::{artifexian::ArtifexianBuilder, Generator},
    program::ProgramBuilder,
    Float,
};
use gloo_utils::format::JsValueSerdeExt;
use rand::SeedableRng;
use rand_xorshift::XorShiftRng;
use serde::Deserialize;
use wasm_bindgen::prelude::*;

use output::Web;
mod output;

// TODO: add support for web workers

//pub use wasm_bindgen_rayon::init_thread_pool;

/// # Errors
/// Returns an error if root or observatories are not valid representations of their values i.e. missing
/// required fields
#[wasm_bindgen]
pub fn generate_observations_from_json(
    root: &str,
    observatories: &str,
    start_time: i128,
    end_time: i128,
    step_size: Option<usize>,
) -> Result<(), JsError> {
    #[cfg(debug_assertions)]
    wasm_log::init(wasm_log::Config::default());

    // Create root body (and whole body tree)
    let fake_root: self::Body = serde_json::from_str(root)?;
    let root = astrograph::body::Body::from(fake_root).into();

    astrograph::body::Body::hydrate_all(&root, &None);

    // Create weak observatories to avoid memory duplication
    let observatories: Vec<WeakObservatory> = serde_json::from_str(observatories)?;

    // Upgrade weak observatories
    let observatories: Vec<Observatory> = observatories
        .into_iter()
        .map(|o| observatory::to_observatory(o, &root))
        .collect();

    // Avoid potential zero step size
    let step_size = step_size.filter(|x| *x != 0);

    // Create program that outputs to the page
    let program = ProgramBuilder::default()
        .root_body(root)
        .outputs(vec![Box::new(Web::default())])
        .observatories(observatories)
        .build()?;

    program.make_observations(start_time, end_time, step_size);
    Ok(())
}

/// Generates a universe from the given seed
#[cfg_attr(any(target_arch = "wasm32", target_arch = "wasm64"), wasm_bindgen)]
#[must_use]
#[allow(clippy::missing_panics_doc)] // Should not be able to panic
pub fn generate_universe_from_seed(seed: u64) -> JsValue {
    #[cfg(debug_assertions)]
    wasm_log::init(wasm_log::Config::default());

    JsValue::from_serde(
        &ArtifexianBuilder::default()
            .star_count(1)
            .build()
            .unwrap()
            .generate(&mut XorShiftRng::seed_from_u64(seed)),
    )
    .unwrap()
}

/// Generates a universe with a random seed
#[cfg_attr(any(target_arch = "wasm32", target_arch = "wasm64"), wasm_bindgen)]
#[must_use]
#[allow(clippy::missing_panics_doc)] // Should not be able to panic
pub fn generate_universe() -> JsValue {
    #[cfg(debug_assertions)]
    wasm_log::init(wasm_log::Config::default());

    JsValue::from_serde(
        &ArtifexianBuilder::default()
            .star_count(100)
            .build()
            .unwrap()
            .generate(&mut XorShiftRng::from_entropy()),
    )
    .unwrap()
}

/// A kind of hacky solution to the problem of serde json not recognizing typetaged dynamics when
/// targeting a wasm arch
#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Body {
    /// Bodies that orbit around this body
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    children: Vec<Body>,
    /// The way this body moves around the parent
    dynamic: Dynamic,
    /// If the body has any o1fservatories it is highly recommended to initialize this.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(default)]
    rotation: Option<Rotating>,
    // Getting some parameters ready for a next version
    // /// Mass of the body in jupiter masses
    //mass: Float,
    #[serde(skip_serializing_if = "Option::is_none")]
    radius: Option<crate::Float>,
    //color: [u8,h8,u8],
    #[serde(skip_serializing_if = "Option::is_none")]
    name: Option<String>,
}

impl From<Body> for astrograph::body::Body {
    fn from(value: Body) -> Self {
        match astrograph::body::BodyBuilder::default()
            .parent(None)
            .name(value.name.map_or(astrograph::body::Name::Unknown, |n| {
                astrograph::body::Name::Named(n.into())
            }))
            .children(
                value
                    .children
                    .into_iter()
                    .map(|c| astrograph::body::Body::from(c).into())
                    .collect(),
            )
            .radius(value.radius)
            .rotation(value.rotation)
            .dynamic(match value.dynamic {
                Dynamic::Fixed(f) => Box::new(f),
                Dynamic::Keplerian(f) => Box::new(f),
            })
            .build()
        {
            Ok(b) => b,
            Err(_) => unreachable!("All generated bodies are valid"),
        }
    }
}

/// List of dynamics that are supported
#[derive(Clone, Debug, Deserialize)]
#[non_exhaustive]
enum Dynamic {
    /// Fixed bodies
    Fixed(Fixed),
    /// Keplerian bodies
    Keplerian(Keplerian),
}

#[cfg(test)]
mod test {
    use super::*;
    use coordinates::prelude::*;
    use wasm_bindgen_test::wasm_bindgen_test;

    #[wasm_bindgen_test(unsupported = test)]
    fn body_conversion() {
        let wasm_body = Body {
            children: vec![],
            dynamic: Dynamic::Fixed(Fixed::new(Vector3::<astrograph::Float>::ORIGIN)),
            rotation: None,
            radius: None,
            name: None,
        };

        let body: astrograph::body::Body = wasm_body.into();

        assert_eq!(body.get_children().len(), 0);
        match body.get_dynamic().as_any().downcast_ref::<Fixed>() {
            Some(a) => assert_eq!(a, &Fixed::new(Vector3::ORIGIN)),
            _ => unreachable!("Should be a fixed dynamic"),
        }

        assert_eq!(body.get_id().len(), 0);
    }

    #[wasm_bindgen_test]
    #[allow(dead_code)] // code is used in wasm-pack test ...
    fn universe_generation() {
        let universe_a = generate_universe_from_seed(0xffff_1111_0000_8888);
        let universe_b = generate_universe_from_seed(0xffff_1111_0000_8888);
        assert_eq!(format!("{universe_a:?}"), format!("{universe_b:?}"));

        let universe_c = generate_universe();
        assert!(universe_c.is_object());
    }
}

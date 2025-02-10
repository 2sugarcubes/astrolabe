use serde::{Deserialize, Serialize};

use log::{trace, warn};

#[derive(Clone, Debug, Deserialize, Serialize, Eq, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Weak {
    /// List of the edges and the IDs of the bodies that mark their ends
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    edges: Vec<(Vec<usize>, Vec<usize>)>,
}

impl Weak {
    pub fn upgrade(self, root: &crate::body::Arc) -> super::Constellation {
        let mut new_edges = Vec::with_capacity(self.edges.len());

        for (a, b) in self.edges {
            if let Some(body_a) = get_body_by_id(&a, root) {
                if let Some(body_b) = get_body_by_id(&b, root) {
                    new_edges.push((body_a, body_b));
                }
            }
        }

        super::Constellation { edges: new_edges }
    }
}

/// Gets a body from the tree based on the ID of the body
fn get_body_by_id(id: &[usize], root: &crate::body::Arc) -> Option<crate::body::Arc> {
    trace!("id = {id:?}, body = {:?}", root.read().unwrap().get_name());
    if id.is_empty() {
        Some(root.clone())
    } else if let Ok(next) = &root
        .read()
        .map(|x| x.get_children()[*id.last().unwrap()].clone())
    {
        get_body_by_id(&id[1..], next)
    } else {
        warn!("Could not find body");
        None
    }
}

impl From<super::Constellation> for Weak {
    fn from(value: super::Constellation) -> Self {
        let edges = value
            .edges
            .iter()
            .filter_map(|(a, b)| {
                a.read()
                    .and_then(|body_a| {
                        b.read()
                            .map(|body_b| (body_a.get_id(), body_b.get_id()))
                            .inspect_err(|e| {
                                warn!("Poison lock while reading body {e:?}, did a thread panic?");
                            })
                    })
                    .inspect_err(|e| {
                        warn!("Poisoned lock while reading body {e:?}, did a thread panic?");
                    })
                    .ok()
            })
            .collect();

        Self { edges }
    }
}

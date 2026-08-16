use rune_core::{EdgeKind, NodeId};
use rune_graph::Graph;
use rune_storage::Store;
use serde::{Deserialize, Serialize};

use crate::error::Result;
use crate::model::{Requirement, Specification};
use crate::store::SpecStore;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct RequirementCoverage {
    pub requirement: Requirement,
    pub specification_id: NodeId,
    pub implementing_nodes: Vec<NodeId>,
    pub violating_nodes: Vec<NodeId>,
    pub covered: bool,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct CoverageReport {
    pub specification_id: NodeId,
    pub requirements: Vec<RequirementCoverage>,
    pub uncovered: Vec<Requirement>,
}

pub struct Coverage<'a> {
    store: &'a Store,
}

impl<'a> Coverage<'a> {
    pub fn new(store: &'a Store) -> Self {
        Self { store }
    }

    pub fn for_specification(&self, spec_id: NodeId) -> Result<CoverageReport> {
        let spec = SpecStore::new(self.store).get(spec_id)?;
        self.report(&spec)
    }

    pub fn report(&self, spec: &Specification) -> Result<CoverageReport> {
        let mut requirements = Vec::new();
        let mut uncovered = Vec::new();
        for requirement in &spec.requirements {
            let coverage = self.requirement_coverage(spec.id, requirement)?;
            if !coverage.covered {
                uncovered.push(requirement.clone());
            }
            requirements.push(coverage);
        }
        Ok(CoverageReport {
            specification_id: spec.id,
            requirements,
            uncovered,
        })
    }

    pub fn uncovered_requirements(&self, spec_id: NodeId) -> Result<Vec<Requirement>> {
        Ok(self.for_specification(spec_id)?.uncovered)
    }

    fn requirement_coverage(
        &self,
        spec_id: NodeId,
        requirement: &Requirement,
    ) -> Result<RequirementCoverage> {
        let mut implementing = Vec::new();
        let mut violating = Vec::new();
        for neighbor in Graph::new(self.store).neighbors(requirement.id)? {
            if neighbor.outgoing {
                continue;
            }
            match neighbor.edge.kind {
                EdgeKind::SatisfiesRequirement | EdgeKind::ImplementsSpec | EdgeKind::VerifiedBy => {
                    implementing.push(neighbor.node.id);
                }
                EdgeKind::ViolatesRequirement => violating.push(neighbor.node.id),
                _ => {}
            }
        }
        implementing.sort_by_key(|id| id.to_string());
        implementing.dedup();
        let covered = !implementing.is_empty();
        Ok(RequirementCoverage {
            requirement: requirement.clone(),
            specification_id: spec_id,
            implementing_nodes: implementing,
            violating_nodes: violating,
            covered,
        })
    }
}

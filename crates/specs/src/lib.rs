//! Structured specifications with individually addressable requirements.

mod coverage;
mod error;
mod model;
mod store;

pub use coverage::{Coverage, CoverageReport, RequirementCoverage};
pub use error::{Result, SpecError};
pub use model::{Requirement, RequirementStatus, SpecStatus, Specification};
pub use store::{new_requirement, new_specification, SpecStore};

#[cfg(test)]
mod tests {
    use super::*;
    use rune_core::{Node, NodeKind};
    use rune_storage::Store;

    #[test]
    fn requirement_with_no_evidence_listed_as_uncovered() {
        let store = Store::open_in_memory().unwrap();
        let mut spec = new_specification("auth", "Need durable sessions");
        spec.current_behavior = "in-memory sessions".into();
        spec.desired_behavior = "persistent sessions".into();
        spec.constraints = vec!["must not store secrets in plaintext".into()];
        spec.nonrequirements = vec!["SSO".into()];
        spec.acceptance_criteria = vec!["sessions survive restart".into()];
        spec.open_questions = vec!["which store?".into()];
        let uncovered_req = new_requirement("REQ_1", "Sessions persist across restart");
        let covered_req = new_requirement("REQ_2", "Passwords are hashed");
        spec.requirements = vec![uncovered_req.clone(), covered_req.clone()];
        let specs = SpecStore::new(&store);
        let spec = specs.create(spec).unwrap();
        let test_node = Node::new(NodeKind::Test, Some("hash_test".into()), serde_json::json!({}));
        store.upsert_node(&test_node).unwrap();
        specs
            .link_satisfies_requirement(test_node.id, spec.requirements[1].id)
            .unwrap();
        let report = Coverage::new(&store).for_specification(spec.id).unwrap();
        assert_eq!(report.uncovered.len(), 1);
        assert_eq!(report.uncovered[0].key, "REQ_1");
        assert!(report.requirements.iter().any(|item| item.requirement.key == "REQ_2" && item.covered));
        let loaded = specs.get_requirement(spec.requirements[0].id).unwrap();
        assert_eq!(loaded.text, "Sessions persist across restart");
    }
}

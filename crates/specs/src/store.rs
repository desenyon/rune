use rune_core::{Edge, EdgeKind, Node, NodeId, NodeKind, Timestamp};
use rune_security::UntrustedContent;
use rune_storage::Store;

use crate::error::{Result, SpecError};
use crate::model::{
    requirement_from_node, spec_from_node, spec_payload, Requirement, RequirementPayload, RequirementStatus,
    SpecStatus, Specification,
};

pub struct SpecStore<'a> {
    store: &'a Store,
}

impl<'a> SpecStore<'a> {
    pub fn new(store: &'a Store) -> Self {
        Self { store }
    }

    pub fn create(&self, mut spec: Specification) -> Result<Specification> {
        validate(&spec)?;
        let problem = UntrustedContent::wrap("specification.problem", &spec.problem);
        let _ = problem.as_instruction();
        spec.problem = problem.body;
        spec.updated_at = Timestamp::now();
        let payload =
            serde_json::to_value(spec_payload(&spec)).map_err(|err| SpecError::msg(err.to_string()))?;
        let mut node = Node::new(NodeKind::Specification, Some(spec.name.clone()), payload);
        node.id = spec.id;
        node.created_at = spec.created_at;
        node.updated_at = spec.updated_at;
        self.store.upsert_node(&node)?;
        for requirement in &spec.requirements {
            self.upsert_requirement(spec.id, requirement)?;
        }
        for component in &spec.affected_components {
            if self
                .store
                .find_edge(spec.id, *component, EdgeKind::Affects)?
                .is_none()
            {
                self.store
                    .upsert_edge(&Edge::new(spec.id, *component, EdgeKind::Affects))?;
            }
        }
        self.get(spec.id)
    }

    pub fn upsert_requirement(&self, spec_id: NodeId, requirement: &Requirement) -> Result<()> {
        if requirement.key.trim().is_empty() {
            return Err(SpecError::invalid("requirement key must not be empty"));
        }
        if requirement.text.trim().is_empty() {
            return Err(SpecError::invalid(format!(
                "requirement {} text must not be empty",
                requirement.key
            )));
        }
        let payload = serde_json::to_value(RequirementPayload {
            key: requirement.key.clone(),
            text: requirement.text.clone(),
            status: requirement.status.clone(),
            specification_id: spec_id,
        })
        .map_err(|err| SpecError::msg(err.to_string()))?;
        let mut node = Node::new(
            NodeKind::Requirement,
            Some(requirement.key.clone()),
            payload,
        );
        node.id = requirement.id;
        self.store.upsert_node(&node)?;
        if self
            .store
            .find_edge(spec_id, requirement.id, EdgeKind::Contains)?
            .is_none()
        {
            self.store
                .upsert_edge(&Edge::new(spec_id, requirement.id, EdgeKind::Contains))?;
        }
        Ok(())
    }

    pub fn list(&self) -> Result<Vec<Specification>> {
        let mut out = Vec::new();
        for node in self.store.nodes_of_kind(NodeKind::Specification)? {
            out.push(self.get(node.id)?);
        }
        Ok(out)
    }

    pub fn get(&self, id: NodeId) -> Result<Specification> {
        let node = self.store.get_node(id)?;
        if node.kind != NodeKind::Specification {
            return Err(SpecError::invalid(format!("{id} is not a specification")));
        }
        let mut requirements = Vec::new();
        for edge in self.store.edges_from_kind(id, EdgeKind::Contains)? {
            let child = self.store.get_node(edge.to)?;
            if child.kind == NodeKind::Requirement {
                requirements.push(
                    requirement_from_node(&child).map_err(|err| SpecError::msg(err.to_string()))?,
                );
            }
        }
        spec_from_node(&node, requirements).map_err(|err| SpecError::msg(err.to_string()))
    }

    pub fn get_requirement(&self, id: NodeId) -> Result<Requirement> {
        let node = self.store.get_node(id)?;
        if node.kind != NodeKind::Requirement {
            return Err(SpecError::invalid(format!("{id} is not a requirement")));
        }
        requirement_from_node(&node).map_err(|err| SpecError::msg(err.to_string()))
    }

    pub fn link_implements_spec(&self, implementer: NodeId, spec_id: NodeId) -> Result<()> {
        self.ensure_kind(spec_id, NodeKind::Specification)?;
        if self
            .store
            .find_edge(implementer, spec_id, EdgeKind::ImplementsSpec)?
            .is_none()
        {
            self.store
                .upsert_edge(&Edge::new(implementer, spec_id, EdgeKind::ImplementsSpec))?;
        }
        Ok(())
    }

    pub fn link_satisfies_requirement(&self, evidence: NodeId, requirement_id: NodeId) -> Result<()> {
        self.ensure_kind(requirement_id, NodeKind::Requirement)?;
        if self
            .store
            .find_edge(evidence, requirement_id, EdgeKind::SatisfiesRequirement)?
            .is_none()
        {
            self.store.upsert_edge(&Edge::new(
                evidence,
                requirement_id,
                EdgeKind::SatisfiesRequirement,
            ))?;
        }
        Ok(())
    }

    pub fn link_violates_requirement(&self, evidence: NodeId, requirement_id: NodeId) -> Result<()> {
        self.ensure_kind(requirement_id, NodeKind::Requirement)?;
        if self
            .store
            .find_edge(evidence, requirement_id, EdgeKind::ViolatesRequirement)?
            .is_none()
        {
            self.store.upsert_edge(&Edge::new(
                evidence,
                requirement_id,
                EdgeKind::ViolatesRequirement,
            ))?;
        }
        Ok(())
    }

    fn ensure_kind(&self, id: NodeId, kind: NodeKind) -> Result<()> {
        let node = self.store.get_node(id)?;
        if node.kind != kind {
            return Err(SpecError::invalid(format!(
                "expected {kind} node {id}, found {}",
                node.kind
            )));
        }
        Ok(())
    }
}

pub fn new_specification(name: impl Into<String>, problem: impl Into<String>) -> Specification {
    let now = Timestamp::now();
    Specification {
        id: NodeId::generate(),
        name: name.into(),
        problem: problem.into(),
        current_behavior: String::new(),
        desired_behavior: String::new(),
        requirements: Vec::new(),
        nonrequirements: Vec::new(),
        constraints: Vec::new(),
        acceptance_criteria: Vec::new(),
        affected_components: Vec::new(),
        open_questions: Vec::new(),
        status: SpecStatus::Draft,
        created_at: now,
        updated_at: now,
    }
}

pub fn new_requirement(key: impl Into<String>, text: impl Into<String>) -> Requirement {
    Requirement {
        id: NodeId::generate(),
        key: key.into(),
        text: text.into(),
        status: RequirementStatus::Open,
    }
}

fn validate(spec: &Specification) -> Result<()> {
    if spec.name.trim().is_empty() {
        return Err(SpecError::invalid("specification name must not be empty"));
    }
    if spec.problem.trim().is_empty() {
        return Err(SpecError::invalid("specification problem must not be empty"));
    }
    let mut keys = std::collections::BTreeSet::new();
    for requirement in &spec.requirements {
        if !keys.insert(requirement.key.clone()) {
            return Err(SpecError::invalid(format!(
                "duplicate requirement key {}",
                requirement.key
            )));
        }
    }
    Ok(())
}

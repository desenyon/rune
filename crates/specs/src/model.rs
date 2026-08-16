use rune_core::{Node, NodeId, Timestamp};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SpecStatus {
    Draft,
    Active,
    Implemented,
    Deprecated,
    Archived,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RequirementStatus {
    Open,
    InProgress,
    Satisfied,
    Violated,
    Deferred,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Requirement {
    pub id: NodeId,
    pub key: String,
    pub text: String,
    pub status: RequirementStatus,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Specification {
    pub id: NodeId,
    pub name: String,
    pub problem: String,
    pub current_behavior: String,
    pub desired_behavior: String,
    pub requirements: Vec<Requirement>,
    pub nonrequirements: Vec<String>,
    pub constraints: Vec<String>,
    pub acceptance_criteria: Vec<String>,
    pub affected_components: Vec<NodeId>,
    pub open_questions: Vec<String>,
    pub status: SpecStatus,
    pub created_at: Timestamp,
    pub updated_at: Timestamp,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SpecPayload {
    pub problem: String,
    pub current_behavior: String,
    pub desired_behavior: String,
    pub nonrequirements: Vec<String>,
    pub constraints: Vec<String>,
    pub acceptance_criteria: Vec<String>,
    pub affected_components: Vec<NodeId>,
    pub open_questions: Vec<String>,
    pub status: SpecStatus,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct RequirementPayload {
    pub key: String,
    pub text: String,
    pub status: RequirementStatus,
    pub specification_id: NodeId,
}

pub fn spec_from_node(node: &Node, requirements: Vec<Requirement>) -> Result<Specification, serde_json::Error> {
    let payload: SpecPayload = serde_json::from_value(node.payload.clone())?;
    Ok(Specification {
        id: node.id,
        name: node.name.clone().unwrap_or_default(),
        problem: payload.problem,
        current_behavior: payload.current_behavior,
        desired_behavior: payload.desired_behavior,
        requirements,
        nonrequirements: payload.nonrequirements,
        constraints: payload.constraints,
        acceptance_criteria: payload.acceptance_criteria,
        affected_components: payload.affected_components,
        open_questions: payload.open_questions,
        status: payload.status,
        created_at: node.created_at,
        updated_at: node.updated_at,
    })
}

pub fn spec_payload(spec: &Specification) -> SpecPayload {
    SpecPayload {
        problem: spec.problem.clone(),
        current_behavior: spec.current_behavior.clone(),
        desired_behavior: spec.desired_behavior.clone(),
        nonrequirements: spec.nonrequirements.clone(),
        constraints: spec.constraints.clone(),
        acceptance_criteria: spec.acceptance_criteria.clone(),
        affected_components: spec.affected_components.clone(),
        open_questions: spec.open_questions.clone(),
        status: spec.status.clone(),
    }
}

pub fn requirement_from_node(node: &Node) -> Result<Requirement, serde_json::Error> {
    let payload: RequirementPayload = serde_json::from_value(node.payload.clone())?;
    Ok(Requirement {
        id: node.id,
        key: payload.key,
        text: payload.text,
        status: payload.status,
    })
}

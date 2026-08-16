use rune_core::{NodeId, ProvenanceSource};
use rune_security::UntrustedContent;
use serde_json::Value;

use crate::error::{MemoryError, Result};
use crate::model::{
    wrap_evidence, ClaimKind, ExtractedClaim, MemoryCategory, MemoryEvidence, MemoryScope,
};

pub struct Extractor;

impl Extractor {
    pub fn from_session_json(value: &Value) -> Result<Vec<ExtractedClaim>> {
        let session_id = value
            .get("session_id")
            .and_then(Value::as_str)
            .ok_or_else(|| MemoryError::invalid("session JSON requires session_id"))?
            .to_string();
        let provider = value
            .get("provider")
            .and_then(Value::as_str)
            .map(ToString::to_string);
        let turns = value
            .get("turns")
            .and_then(Value::as_array)
            .ok_or_else(|| MemoryError::invalid("session JSON requires turns[]"))?;
        let mut claims = Vec::new();
        for turn in turns {
            let role = turn
                .get("role")
                .and_then(Value::as_str)
                .ok_or_else(|| MemoryError::invalid("session turn requires role"))?;
            let turn_id = turn
                .get("id")
                .and_then(Value::as_str)
                .map(ToString::to_string);
            let content = turn.get("content").and_then(Value::as_str).unwrap_or("");
            let wrapped = UntrustedContent::wrap(format!("session:{session_id}"), content);
            let _ = wrapped.as_instruction();
            if let Some(explicit) = turn.get("claims").and_then(Value::as_array) {
                for claim in explicit {
                    claims.push(claim_from_json(
                        claim,
                        role,
                        &session_id,
                        turn_id.clone(),
                        provider.clone(),
                    )?);
                }
                continue;
            }
            if let Some(claim) = infer_from_turn(role, &wrapped.body, &session_id, turn_id.clone(), provider.clone(), turn)?
            {
                claims.push(claim);
            }
        }
        Ok(claims)
    }

    pub fn from_commit(sha: &str, message: &str) -> Result<Vec<ExtractedClaim>> {
        if sha.trim().is_empty() {
            return Err(MemoryError::invalid("commit sha must not be empty"));
        }
        if message.trim().is_empty() {
            return Err(MemoryError::invalid("commit message must not be empty"));
        }
        let wrapped = UntrustedContent::wrap(format!("commit:{sha}"), message);
        let _ = wrapped.as_instruction();
        let statement = first_line(&wrapped.body);
        Ok(vec![ExtractedClaim {
            statement,
            claim_kind: ClaimKind::ObservedFact,
            category: MemoryCategory::WorkflowConvention,
            scope: MemoryScope::Repository,
            confidence: 0.6,
            evidence: vec![wrap_evidence(
                "git_commit",
                &wrapped.body,
                ProvenanceSource::GitCommit { sha: sha.to_string() },
            )],
            related_nodes: Vec::new(),
            actor: None,
        }])
    }

    pub fn from_specification(spec_id: &str, body: &Value) -> Result<Vec<ExtractedClaim>> {
        if spec_id.trim().is_empty() {
            return Err(MemoryError::invalid("specification id must not be empty"));
        }
        let mut claims = Vec::new();
        if let Some(constraints) = body.get("constraints").and_then(Value::as_array) {
            for constraint in constraints {
                let text = constraint
                    .as_str()
                    .or_else(|| constraint.get("text").and_then(Value::as_str))
                    .ok_or_else(|| MemoryError::invalid("specification constraint must be text"))?;
                claims.push(ExtractedClaim {
                    statement: text.to_string(),
                    claim_kind: ClaimKind::ObservedFact,
                    category: MemoryCategory::ProjectConstraint,
                    scope: MemoryScope::Project,
                    confidence: 0.9,
                    evidence: vec![wrap_evidence(
                        "specification",
                        text,
                        ProvenanceSource::Specification {
                            spec_id: spec_id.to_string(),
                            requirement_id: None,
                        },
                    )],
                    related_nodes: Vec::new(),
                    actor: None,
                });
            }
        }
        if claims.is_empty() {
            return Err(MemoryError::invalid(
                "specification contained no extractable constraints",
            ));
        }
        Ok(claims)
    }

    pub fn from_human_statement(
        actor: &str,
        statement: &str,
        kind: Option<ClaimKind>,
    ) -> Result<ExtractedClaim> {
        if actor.trim().is_empty() {
            return Err(MemoryError::invalid("human statement requires an actor"));
        }
        if statement.trim().is_empty() {
            return Err(MemoryError::invalid("human statement must not be empty"));
        }
        let wrapped = UntrustedContent::wrap(format!("human:{actor}"), statement);
        let _ = wrapped.as_instruction();
        let claim_kind = kind.unwrap_or_else(|| classify_human(&wrapped.body));
        Ok(ExtractedClaim {
            statement: wrapped.body.clone(),
            claim_kind: claim_kind.clone(),
            category: match claim_kind {
                ClaimKind::HumanPreference => MemoryCategory::DeveloperPreference,
                ClaimKind::TemporaryAssumption => MemoryCategory::TemporaryContext,
                _ => MemoryCategory::ArchitecturalDecision,
            },
            scope: MemoryScope::User,
            confidence: 1.0,
            evidence: vec![wrap_evidence(
                "human_input",
                &wrapped.body,
                ProvenanceSource::HumanInput {
                    actor: actor.to_string(),
                },
            )],
            related_nodes: Vec::new(),
            actor: Some(actor.to_string()),
        })
    }
}

fn claim_from_json(
    claim: &Value,
    role: &str,
    session_id: &str,
    turn_id: Option<String>,
    provider: Option<String>,
) -> Result<ExtractedClaim> {
    let statement = claim
        .get("statement")
        .and_then(Value::as_str)
        .ok_or_else(|| MemoryError::invalid("claim requires statement"))?;
    let mut claim_kind = parse_claim_kind(claim.get("kind").and_then(Value::as_str))?;
    if is_agent_role(role) {
        if matches!(claim_kind, ClaimKind::ObservedFact | ClaimKind::HumanPreference) {
            claim_kind = ClaimKind::AgentInference;
        }
    }
    if claim.get("guess").and_then(Value::as_bool) == Some(true)
        || claim.get("inference").and_then(Value::as_bool) == Some(true)
    {
        claim_kind = ClaimKind::AgentInference;
    }
    let category = parse_category(claim.get("category").and_then(Value::as_str), &claim_kind)?;
    let related_nodes = parse_related(claim.get("related_nodes"))?;
    Ok(ExtractedClaim {
        statement: statement.to_string(),
        claim_kind: claim_kind.clone(),
        category,
        scope: MemoryScope::Session,
        confidence: claim
            .get("confidence")
            .and_then(Value::as_f64)
            .map(|value| value as f32)
            .unwrap_or(if matches!(claim_kind, ClaimKind::AgentInference) {
                0.4
            } else {
                0.8
            }),
        evidence: vec![MemoryEvidence {
            source: ProvenanceSource::AgentSession {
                session_id: session_id.to_string(),
                turn_id,
                provider,
            },
            excerpt: Some(statement.to_string()),
            observed_at: rune_core::Timestamp::now(),
            derived: matches!(claim_kind, ClaimKind::AgentInference),
            confidence: if matches!(claim_kind, ClaimKind::AgentInference) {
                0.4
            } else {
                0.8
            },
        }],
        related_nodes,
        actor: None,
    })
}

fn infer_from_turn(
    role: &str,
    body: &str,
    session_id: &str,
    turn_id: Option<String>,
    provider: Option<String>,
    turn: &Value,
) -> Result<Option<ExtractedClaim>> {
    if body.trim().is_empty() {
        return Ok(None);
    }
    let flagged_guess = turn.get("guess").and_then(Value::as_bool) == Some(true)
        || turn.get("inference").and_then(Value::as_bool) == Some(true)
        || looks_like_guess(body);
    let claim_kind = if is_agent_role(role) || flagged_guess {
        if looks_like_assumption(body) {
            ClaimKind::TemporaryAssumption
        } else {
            ClaimKind::AgentInference
        }
    } else {
        classify_human(body)
    };
    let category = match claim_kind {
        ClaimKind::HumanPreference => MemoryCategory::DeveloperPreference,
        ClaimKind::TemporaryAssumption => MemoryCategory::TemporaryContext,
        ClaimKind::AgentInference => MemoryCategory::TemporaryContext,
        ClaimKind::ObservedFact => MemoryCategory::VerifiedFact,
    };
    Ok(Some(ExtractedClaim {
        statement: first_line(body),
        claim_kind: claim_kind.clone(),
        category,
        scope: MemoryScope::Session,
        confidence: if matches!(claim_kind, ClaimKind::AgentInference) {
            0.35
        } else {
            0.8
        },
        evidence: vec![MemoryEvidence {
            source: if is_agent_role(role) {
                ProvenanceSource::DerivedInference {
                    method: "session_turn".into(),
                    inputs: vec![session_id.to_string()],
                }
            } else {
                ProvenanceSource::AgentSession {
                    session_id: session_id.to_string(),
                    turn_id,
                    provider,
                }
            },
            excerpt: Some(body.to_string()),
            observed_at: rune_core::Timestamp::now(),
            derived: is_agent_role(role) || flagged_guess,
            confidence: 0.4,
        }],
        related_nodes: Vec::new(),
        actor: None,
    }))
}

fn parse_claim_kind(value: Option<&str>) -> Result<ClaimKind> {
    match value {
        None => Ok(ClaimKind::ObservedFact),
        Some("observed_fact") => Ok(ClaimKind::ObservedFact),
        Some("human_preference") => Ok(ClaimKind::HumanPreference),
        Some("agent_inference") => Ok(ClaimKind::AgentInference),
        Some("temporary_assumption") => Ok(ClaimKind::TemporaryAssumption),
        Some(other) => Err(MemoryError::invalid(format!("unknown claim kind `{other}`"))),
    }
}

fn parse_category(value: Option<&str>, kind: &ClaimKind) -> Result<MemoryCategory> {
    match value {
        None => Ok(match kind {
            ClaimKind::HumanPreference => MemoryCategory::DeveloperPreference,
            ClaimKind::TemporaryAssumption => MemoryCategory::TemporaryContext,
            ClaimKind::AgentInference => MemoryCategory::TemporaryContext,
            ClaimKind::ObservedFact => MemoryCategory::VerifiedFact,
        }),
        Some("architectural_decision") => Ok(MemoryCategory::ArchitecturalDecision),
        Some("project_constraint") => Ok(MemoryCategory::ProjectConstraint),
        Some("developer_preference") => Ok(MemoryCategory::DeveloperPreference),
        Some("verified_fact") => Ok(MemoryCategory::VerifiedFact),
        Some("workflow_convention") => Ok(MemoryCategory::WorkflowConvention),
        Some("failure_pattern") => Ok(MemoryCategory::FailurePattern),
        Some("successful_procedure") => Ok(MemoryCategory::SuccessfulProcedure),
        Some("environment_detail") => Ok(MemoryCategory::EnvironmentDetail),
        Some("temporary_context") => Ok(MemoryCategory::TemporaryContext),
        Some("external_dependency_fact") => Ok(MemoryCategory::ExternalDependencyFact),
        Some(other) => Err(MemoryError::invalid(format!("unknown memory category `{other}`"))),
    }
}

fn parse_related(value: Option<&Value>) -> Result<Vec<NodeId>> {
    let Some(Value::Array(items)) = value else {
        return Ok(Vec::new());
    };
    let mut ids = Vec::new();
    for item in items {
        let text = item
            .as_str()
            .ok_or_else(|| MemoryError::invalid("related_nodes entries must be node id strings"))?;
        ids.push(
            text.parse()
                .map_err(|err| MemoryError::invalid(format!("invalid related node id: {err}")))?,
        );
    }
    Ok(ids)
}

fn is_agent_role(role: &str) -> bool {
    matches!(
        role.to_ascii_lowercase().as_str(),
        "assistant" | "agent" | "model" | "system"
    )
}

fn classify_human(body: &str) -> ClaimKind {
    let lower = body.to_ascii_lowercase();
    if looks_like_assumption(&lower) {
        ClaimKind::TemporaryAssumption
    } else if lower.contains("prefer")
        || lower.contains("always ")
        || lower.contains("never ")
        || lower.contains("must ")
        || lower.contains("do not ")
    {
        ClaimKind::HumanPreference
    } else {
        ClaimKind::ObservedFact
    }
}

fn looks_like_guess(body: &str) -> bool {
    let lower = body.to_ascii_lowercase();
    lower.contains("i think")
        || lower.contains("might")
        || lower.contains("probably")
        || lower.contains("guess")
        || lower.contains("perhaps")
        || lower.contains("it seems")
}

fn looks_like_assumption(body: &str) -> bool {
    let lower = body.to_ascii_lowercase();
    lower.contains("assume") || lower.contains("for now") || lower.contains("temporarily")
}

fn first_line(body: &str) -> String {
    body.lines()
        .next()
        .unwrap_or(body)
        .trim()
        .to_string()
}

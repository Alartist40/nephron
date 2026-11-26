//! Multi-agent swarm cursor spring physics & Cynapse event-stream state.

pub mod actions;
pub mod canvas;

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum AgentRole {
    Researcher,
    Builder,
    Reviewer,
    CadSmith,
}

impl AgentRole {
    pub fn label(self) -> &'static str {
        match self {
            Self::Researcher => "Researcher",
            Self::Builder => "Builder",
            Self::Reviewer => "Reviewer",
            Self::CadSmith => "CAD-Smith",
        }
    }

    pub fn rgb(self) -> [u8; 3] {
        match self {
            Self::Researcher => [90, 200, 250],
            Self::Builder => [120, 230, 140],
            Self::Reviewer => [255, 190, 90],
            Self::CadSmith => [220, 130, 255],
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SwarmAction {
    Read,
    Edit,
    Search,
    CadBuild,
    Test,
    Idle,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SwarmEvent {
    pub id: String,
    pub role: AgentRole,
    pub action: SwarmAction,
    pub target: [f32; 2],
    pub detail: String,
}

#[derive(Debug, Clone)]
pub struct SwarmCursor {
    pub id: String,
    pub role: AgentRole,
    pub action: SwarmAction,
    pub pos: [f32; 2],
    pub vel: [f32; 2],
    pub target: [f32; 2],
    pub detail: String,
    pub phase: f32,
}

impl SwarmCursor {
    pub fn step(&mut self, dt: f32) {
        let stiffness = 95.0;
        let damping = 16.0;
        for i in 0..2 {
            let accel = (self.target[i] - self.pos[i]) * stiffness - self.vel[i] * damping;
            self.vel[i] += accel * dt;
            self.pos[i] = (self.pos[i] + self.vel[i] * dt).clamp(0.02, 0.98);
        }
        self.phase += dt * 4.0;
    }
}

#[derive(Default)]
pub struct SwarmCanvas {
    pub cursors: HashMap<String, SwarmCursor>,
    pub demo_mode: bool,
    demo_clock: f32,
}

impl SwarmCanvas {
    pub fn ingest(&mut self, ev: SwarmEvent) {
        let entry = self.cursors.entry(ev.id.clone()).or_insert_with(|| SwarmCursor {
            id: ev.id,
            role: ev.role,
            action: ev.action,
            pos: ev.target,
            vel: [0.0, 0.0],
            target: ev.target,
            detail: ev.detail.clone(),
            phase: 0.0,
        });
        entry.role = ev.role;
        entry.action = ev.action;
        entry.target = [ev.target[0].clamp(0.05, 0.95), ev.target[1].clamp(0.05, 0.95)];
        entry.detail = ev.detail;
    }

    pub fn tick(&mut self, dt: f32) {
        if self.demo_mode {
            self.demo_clock += dt;
            if self.cursors.is_empty() || (self.demo_clock * 0.6).fract() < dt * 0.6 {
                let t = self.demo_clock;
                self.ingest(SwarmEvent {
                    id: "a1".into(),
                    role: AgentRole::Researcher,
                    action: SwarmAction::Search,
                    target: [0.3 + 0.2 * t.sin(), 0.35 + 0.15 * t.cos()],
                    detail: "rg memory.rs".into(),
                });
                self.ingest(SwarmEvent {
                    id: "a2".into(),
                    role: AgentRole::Builder,
                    action: SwarmAction::Edit,
                    target: [0.6 + 0.18 * (t + 1.5).cos(), 0.5 + 0.2 * (t + 1.0).sin()],
                    detail: "edit app.rs".into(),
                });
                self.ingest(SwarmEvent {
                    id: "a3".into(),
                    role: AgentRole::Reviewer,
                    action: SwarmAction::Test,
                    target: [0.5 + 0.15 * (t + 3.0).sin(), 0.72 + 0.1 * (t + 2.0).cos()],
                    detail: "cargo test".into(),
                });
            }
        }
        for c in self.cursors.values_mut() {
            c.step(dt);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn spring_converges_to_target() {
        let mut canvas = SwarmCanvas::default();
        canvas.ingest(SwarmEvent {
            id: "r1".into(),
            role: AgentRole::Researcher,
            action: SwarmAction::Read,
            target: [0.2, 0.2],
            detail: "read".into(),
        });
        canvas.ingest(SwarmEvent {
            id: "r1".into(),
            role: AgentRole::Researcher,
            action: SwarmAction::Edit,
            target: [0.8, 0.7],
            detail: "edit".into(),
        });
        for _ in 0..120 {
            canvas.tick(1.0 / 60.0);
        }
        let c = &canvas.cursors["r1"];
        assert!((c.pos[0] - 0.8).abs() < 0.02);
        assert!((c.pos[1] - 0.7).abs() < 0.02);
    }
}

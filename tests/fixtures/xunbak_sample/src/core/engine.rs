/// Core engine module — a larger source file for compression ratio testing.
/// Contains repetitive structures that compress well with zstd.

use std::collections::BTreeMap;
use std::io::{self, Write};

/// Represents a processing stage in the pipeline.
#[derive(Debug, Clone)]
pub struct Stage {
    pub name: String,
    pub priority: u32,
    pub enabled: bool,
    pub config: BTreeMap<String, String>,
}

impl Stage {
    pub fn new(name: &str, priority: u32) -> Self {
        Self {
            name: name.to_string(),
            priority,
            enabled: true,
            config: BTreeMap::new(),
        }
    }

    pub fn with_config(mut self, key: &str, value: &str) -> Self {
        self.config.insert(key.to_string(), value.to_string());
        self
    }

    pub fn disable(mut self) -> Self {
        self.enabled = false;
        self
    }
}

/// Pipeline processes stages in priority order.
pub struct Pipeline {
    stages: Vec<Stage>,
}

impl Pipeline {
    pub fn new() -> Self {
        Self { stages: Vec::new() }
    }

    pub fn add_stage(&mut self, stage: Stage) {
        self.stages.push(stage);
        self.stages.sort_by_key(|s| s.priority);
    }

    pub fn run(&self, writer: &mut dyn Write) -> io::Result<()> {
        for stage in &self.stages {
            if !stage.enabled {
                continue;
            }
            writeln!(writer, "[stage:{}] priority={}", stage.name, stage.priority)?;
            for (key, value) in &stage.config {
                writeln!(writer, "  {key} = {value}")?;
            }
        }
        Ok(())
    }

    pub fn stage_count(&self) -> usize {
        self.stages.len()
    }

    pub fn enabled_count(&self) -> usize {
        self.stages.iter().filter(|s| s.enabled).count()
    }
}

/// Statistics collector for pipeline execution metrics.
#[derive(Debug, Default)]
pub struct PipelineStats {
    pub total_stages: u64,
    pub executed_stages: u64,
    pub skipped_stages: u64,
    pub total_duration_ns: u64,
    pub max_stage_duration_ns: u64,
    pub min_stage_duration_ns: u64,
    pub errors: Vec<String>,
}

impl PipelineStats {
    pub fn new() -> Self {
        Self {
            min_stage_duration_ns: u64::MAX,
            ..Default::default()
        }
    }

    pub fn record_execution(&mut self, duration_ns: u64) {
        self.executed_stages += 1;
        self.total_duration_ns += duration_ns;
        self.max_stage_duration_ns = self.max_stage_duration_ns.max(duration_ns);
        self.min_stage_duration_ns = self.min_stage_duration_ns.min(duration_ns);
    }

    pub fn record_skip(&mut self) {
        self.skipped_stages += 1;
    }

    pub fn record_error(&mut self, msg: String) {
        self.errors.push(msg);
    }

    pub fn avg_duration_ns(&self) -> u64 {
        if self.executed_stages == 0 {
            return 0;
        }
        self.total_duration_ns / self.executed_stages
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pipeline_orders_by_priority() {
        let mut p = Pipeline::new();
        p.add_stage(Stage::new("c", 30));
        p.add_stage(Stage::new("a", 10));
        p.add_stage(Stage::new("b", 20));
        assert_eq!(p.stages[0].name, "a");
        assert_eq!(p.stages[1].name, "b");
        assert_eq!(p.stages[2].name, "c");
    }

    #[test]
    fn pipeline_counts_enabled_stages() {
        let mut p = Pipeline::new();
        p.add_stage(Stage::new("x", 1));
        p.add_stage(Stage::new("y", 2).disable());
        p.add_stage(Stage::new("z", 3));
        assert_eq!(p.stage_count(), 3);
        assert_eq!(p.enabled_count(), 2);
    }

    #[test]
    fn stats_averages() {
        let mut s = PipelineStats::new();
        s.record_execution(100);
        s.record_execution(200);
        s.record_execution(300);
        assert_eq!(s.avg_duration_ns(), 200);
        assert_eq!(s.min_stage_duration_ns, 100);
        assert_eq!(s.max_stage_duration_ns, 300);
    }
}

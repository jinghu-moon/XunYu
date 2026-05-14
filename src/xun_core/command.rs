//! CommandSpec — 统一命令 trait
//!
//! 所有 CLI 命令实现 CommandSpec trait，通过 execute() 函数统一调度：
//! validate → before hooks → run → after hooks → render。

use crate::xun_core::context::CmdContext;
use crate::xun_core::error::XunError;
use crate::xun_core::renderer::Renderer;
use crate::xun_core::value::Value;

/// 统一命令规格 trait。
///
/// 每个 CLI 命令实现此 trait，提供 validate（可选）和 run（必须）。
pub trait CommandSpec {
    /// 命令校验（默认通过）。
    fn validate(&self, _ctx: &CmdContext) -> Result<(), XunError> {
        Ok(())
    }

    /// 执行命令，返回结构化输出。
    fn run(&self, ctx: &mut CmdContext) -> Result<Value, XunError>;
}

/// 执行命令：validate → render output。
///
/// 泛型函数，零 vtable 开销。
pub fn execute<C: CommandSpec>(
    cmd: &C,
    ctx: &mut CmdContext,
    renderer: &mut dyn Renderer,
) -> Result<Value, XunError> {
    cmd.validate(ctx)?;
    let output = cmd.run(ctx)?;
    renderer.render_value(&output)?;
    Ok(output)
}

/// Pipeline hook 函数类型。
type HookFn = Box<dyn Fn(&CmdContext) -> Result<(), XunError>>;

/// Pipeline — before/after middleware 集合。
pub struct Pipeline {
    before: Vec<HookFn>,
    after: Vec<HookFn>,
}

impl Default for Pipeline {
    fn default() -> Self {
        Self::new()
    }
}

impl Pipeline {
    /// 创建空 Pipeline。
    pub fn new() -> Self {
        Self {
            before: Vec::new(),
            after: Vec::new(),
        }
    }

    /// 添加 before hook。
    pub fn add_before<F>(&mut self, f: F)
    where
        F: Fn(&CmdContext) -> Result<(), XunError> + 'static,
    {
        self.before.push(Box::new(f));
    }

    /// 添加 after hook。
    pub fn add_after<F>(&mut self, f: F)
    where
        F: Fn(&CmdContext) -> Result<(), XunError> + 'static,
    {
        self.after.push(Box::new(f));
    }

    /// 执行所有 before hooks。
    pub fn run_before(&self, ctx: &CmdContext) -> Result<(), XunError> {
        for hook in &self.before {
            hook(ctx)?;
        }
        Ok(())
    }

    /// 执行所有 after hooks。
    pub fn run_after(&self, ctx: &CmdContext) -> Result<(), XunError> {
        for hook in &self.after {
            hook(ctx)?;
        }
        Ok(())
    }
}

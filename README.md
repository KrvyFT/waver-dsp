# waver-dsp

实时 `Process` 契约与内置节点实现；`for_kind` 是唯一 DSP 工厂（引擎 rebuild 时调用）。

本仓库是 [waver](https://github.com/KrvyFT/waver) workspace 的一部分，在伞仓中位于 `crates/waver-dsp`（git submodule）。

## 仓库

- GitHub：https://github.com/KrvyFT/waver-dsp
- 默认分支：`main`
- License：MIT OR Apache-2.0
- 依赖：[`waver-core`](https://github.com/KrvyFT/waver-core)

## 在伞仓里开发（推荐）

```bash
git clone --recurse-submodules https://github.com/KrvyFT/waver.git
cd waver/crates/waver-dsp
git add -A && git commit -m "…" && git push
cd ../..
./scripts/repos.sh sync
git commit -m "chore: bump waver-dsp" && git push
```

在伞仓根目录：`cargo test -p waver-dsp`。

## 单独 clone

```bash
git clone https://github.com/KrvyFT/waver-dsp.git
```

可独立推送；构建请走伞仓。见 [doc/repos.md](https://github.com/KrvyFT/waver/blob/master/doc/repos.md)。

## 接口文档

[通用 DSP 模块接口](docs/module-interface.md)：缓冲契约、参数与状态、工厂调用、模块实现示例、宿主限制和验证清单。示例随 rustdoc 测试执行。

## 内容概要

| 项 | 说明 |
|----|------|
| `Process` / `ProcessCtx` | `process` 内禁止分配 / 锁 / IO |
| `master_slice` | sink（如 Output）可选主总线 |
| 目录 | `nodes/oscillator`、`filter`、`utility`、`io`（按 `ModuleFamily`） |
| 内置节点 | `Vco`、`Noise`、`Vcf`、`Output`、`Delay`、`Silence`、`Scope` |
| `for_kind` | `NodeKind` → `Box<dyn Process>`；未实现 kind 返回 `None` |

扩展类型 / 模块见伞仓 [doc/modules.md](https://github.com/KrvyFT/waver/blob/master/doc/modules.md)（Noise 为标准示范）。

实时约定见 [doc/audio-thread.md](https://github.com/KrvyFT/waver/blob/master/doc/audio-thread.md)。

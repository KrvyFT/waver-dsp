# 通用 DSP 模块接口

本文描述当前可执行契约及扩展步骤。构建、测试均在 `waver` 伞仓根目录运行；各 crate 通过 workspace/path 依赖联调。

## 1. 接口分层

| 层 | 类型 / 接口 | 职责 |
|---|---|---|
| Core | `NodeKind`、`NodeId` | 模块类型与某个 Patch 内的实例身份 |
| Core | `ModuleFamily`、`ModuleDesc`、`MODULE_CATALOG` | 分类、端口数量、默认参数、UI 文案与可添加状态 |
| Core | `Graph` → `CompiledPatch` | 验证连线，生成 `Schedule` 和共享 `ParamRegistry` |
| DSP | `for_kind` | 将类型、实例 ID、参数单元绑定为 `Box<dyn Process>` |
| DSP | `Process`、`ProcessCtx` | 对借用的音频缓冲执行块处理 |
| Engine | `Engine` | 路由求和、调用处理器、取得主总线并写入设备缓冲 |
| UI | 模块库、检查器 | 只读取 Core 元数据，修改 `ParamCell`；不依赖 DSP |

`ModuleFamily` 是展示和源码组织方式，不是 DSP trait，也不决定算法。一个 family 可包含多个 `NodeKind`；同一种 kind 可在图中拥有多个独立实例。

这是一套**编译期注册的内置模块接口**，目前没有动态插件加载、运行时注册或稳定二进制 ABI。

## 2. 模块描述符与分类

`ModuleFamily::ALL` 决定模块库分组顺序；新增分类须同步枚举、`label()`、`ALL`。

| Family | 分类 | 已实现模块 |
|---|---|---|
| `Oscillator` | 振荡器 | `Vco`、`Noise` |
| `Filter` | 滤波器 | 无，`Vcf` 为计划项 |
| `AmpEnv` | 放大 / 包络 | 无，`Vca`、`Adsr` 为计划项 |
| `Modulation` | 调制 | 无，`Lfo` 为计划项 |
| `Mixer` | 混音 | 无，`Mixer` 为计划项 |
| `Utility` | 工具 | `Delay`、`Silence` |
| `Io` | 输入 / 输出 | `Output` |

`ModuleDesc` 位于 `waver-core/src/module.rs`：

- `kind`、`family`：类型身份与分组。
- `ports: PortCounts`：输入口、输出口、参数数量。三者独立从 0 编号，`PortId(0)` 可以同时表示输入 0 和输出 0，方向由连线端点决定。
- `name`、`code`：模块库文案及搜索字段。
- `canvas_label`、`summary`、`inspector_blurb`：画布和检查器文案。
- `addable`：模块库是否允许添加。它**不阻止**调用方直接 `Graph::insert`。
- `param_defaults`、`param_labels`：顺序对应 `ParamId`；两者长度必须等于 `ports.params`。

`NodeKind::desc()` 通过手写下标访问 `MODULE_CATALOG`。插入、重排目录时必须同步映射；已有测试检查唯一性、覆盖、端口和参数数量。

当前描述符没有参数的最小值、最大值、单位、步长、枚举选项或线性/对数缩放元数据。非 VCO 的通用检查器暂时统一使用 **0..=1 的线性滑条**，适合 Noise 振幅；新增 Hz、秒或枚举参数时，需补充元数据与 UI，或使用专用检查器，不能直接套用该范围。

## 3. 运行时处理契约

```text
pub trait Process: Send {
    fn process(&mut self, ctx: &mut ProcessCtx<'_>);
    fn master_slice(&self) -> Option<&[f32]> { None }
}
```

实现模块时请实现公开的 `waver_dsp::Process`。`Send` 允许实例交给音频线程；不要求 `Sync`，也不应再用 `Mutex` 包裹节点。

### ProcessCtx

| 字段 | 类型 | 调用约定 |
|---|---|---|
| `sample_rate` | `f32` | Hz，宿主提供有限且大于 0 的值 |
| `block` | `usize` | 本次有效帧数；可能短于引擎内部块长 |
| `inputs` | `&[&[f32]]` | 单声道平面输入，总线下标对应输入 `PortId` |
| `outputs` | `&mut [&mut [f32]]` | 独占可写的单声道平面输出，对应输出 `PortId` |

宿主应提供描述符要求的总线数量，每条总线至少有 `block` 个样本。输入缓冲只读；输出必须完整写入 `[..block]`，不得依赖上一调用残留。不要访问或改动有效帧以外的区域，不要保留借用的切片。

当前引擎会为每个输入口准备缓冲：未连接输入为零，多条连线进入同一输入时逐样本求和；不会自动限幅。`inputs.is_empty()` 表示没有输入总线，并不表示一个有输入端口的模块处于未连接状态。

当前宿主上限是 **4 个输入、1 个输出、每次最多 64 帧**。`ProcessCtx` 的切片形状允许更多总线，但 Engine 暂时只向处理器提供并回写输出口 0。声明多输出、超过 4 个输入的模块之前必须扩展宿主路由。`Engine::process_block` 的直接调用方还须确保交错缓冲长度可被设备声道数整除；cpal 路径负责按块切分。

### 状态与参数

- 相位、滤波器记忆、PRNG 等算法状态属于处理器实例，跨 `process` 调用保留。
- 构造时从 `ParamRegistry::get(node, ParamId)` 获取 `Arc<ParamCell>`；缺失时工厂返回 `None`。
- UI 用 `ParamCell::set` 写入；DSP 通常每块调用一次 `value()`，避免每样本重复原子读取。
- `ParamCell` 使用 `Relaxed` 原子读写保存 f32 位模式；它不校验范围、不拒绝 NaN/Inf、不做平滑。调用方应写入有限合法值，需要平滑或防御性处理的模块应自行实现。
- 多个参数不是一次原子快照；有关联的参数需要模块自己定义一致性策略。
- 图重编译时存活节点复用参数单元；当前 Engine 会重新创建**所有**处理器，故相位、噪声序列、Delay 历史会重置。

### 主总线

`Process::master_slice()` 默认返回 `None`。`Output` 是无图输出口的 sink，它把输入复制到内部固定缓冲，再通过 `Some(&[f32])` 暴露最近一块主总线。

当前 Engine 仅查找 `NodeKind::Output`，选择调度序中最后一个 Output；不会把多个 Output 混合。单声道主总线复制到设备各声道，并不构成立体声图。自定义 sink 即使实现 `master_slice()`，也必须同步扩展宿主选择逻辑。

## 4. 工厂、线程和实时边界

```rust
use waver_core::{Graph, NodeKind, ParamId};
use waver_dsp::{for_kind, ProcessCtx};

let mut graph = Graph::new();
let node = graph.insert(NodeKind::Noise);
let patch = graph.compile_patch(None).unwrap();
let mut processor = for_kind(NodeKind::Noise, node, &patch.params).unwrap();
patch.params.get(node, ParamId::new(0)).unwrap().set(0.25);
let mut output = [0.0; 64];
processor.process(&mut ProcessCtx {
    sample_rate: 48_000.0,
    block: 64,
    inputs: &[],
    outputs: &mut [&mut output],
});
assert!(output.iter().all(|sample| sample.abs() <= 0.25));
```

`for_kind(kind, node, params) -> Option<Box<dyn Process>>` 对计划中类型或参数缺失返回 `None`，Engine 用 `Silence` 兜底。工厂不校验给定 ID 是否属于该 kind，调用方必须从同一份 schedule/registry 取值。

实际调用顺序：

1. GUI 编译 `Graph`，生成 `Arc<CompiledPatch>`，入队 `SwapSchedule`。
2. 音频回调 drain 命令，通过 `apply_rt` → `rebuild` → `for_kind` 构造处理器。
3. 每块路由输入、执行 `process`、读取主总线。

**`for_kind` 允许分配，但当前是在音频回调中运行，并非后台编译线程。** 模块的 `process`、`master_slice` 必须无堆分配、无锁、无阻塞、无文件/网络 I/O、无日志。构造阶段应预分配状态，但这不代表当前整个引擎已经满足严格实时要求。

现存宿主限制：

- `rebuild` 构造/销毁节点，`process_block` 克隆执行序，`sources_to` 和输入切片列表创建 `Vec`，仍有回调内分配。
- 自动插入 Delay 的编译测试只验证图能排序。引擎每块清空输出缓存；反馈回边的消费者若先于 Delay 执行，会读到零，尚不能保证正确反馈音频。
- Delay 按块内下标保留上一调用的样本；可变短块不能视为严格固定 64 样本的延时线。
- UI 当前忽略命令队列满时的 `push` 错误，极端连续拓扑编辑可能导致画布与音频调度暂时不一致。
- 主静音、限幅和有效的 `AllNotesOff` 尚未实现；模块默认振幅不是静音（VCO 为 0.5，Noise 为 0.2）。

这些限制属于宿主，不能仅通过实现新的 `Process` 解决。

## 5. 实现一个处理器

下面的 Gain 示例可独立调用，尚未注册为内置 `NodeKind`。它演示参数共享、块边界和断开输入时写零；所有分配发生在构造/测试路径。

```rust
use std::sync::Arc;
use waver_core::ParamCell;
use waver_dsp::{Process, ProcessCtx};

struct Gain {
    gain: Arc<ParamCell>,
}

impl Process for Gain {
    fn process(&mut self, ctx: &mut ProcessCtx<'_>) {
        let raw = self.gain.value();
        let gain = if raw.is_finite() { raw.clamp(0.0, 1.0) } else { 0.0 };
        let out = &mut ctx.outputs[0][..ctx.block];
        if let Some(input) = ctx.inputs.first() {
            for (dst, src) in out.iter_mut().zip(&input[..ctx.block]) {
                *dst = *src * gain;
            }
        } else {
            out.fill(0.0);
        }
    }
}

let cell = Arc::new(ParamCell::new(0.5));
let mut gain = Gain { gain: Arc::clone(&cell) };
let input = [0.8, -0.4, 0.0];
let mut out = [0.0; 3];
gain.process(&mut ProcessCtx {
    sample_rate: 48_000.0, block: 3,
    inputs: &[&input], outputs: &mut [&mut out],
});
assert_eq!(out, [0.4, -0.2, 0.0]);
cell.set(0.0); // 下一块读取新参数，无需重建节点。
```

## 6. 注册一个内置模块

以已有 `Noise` 为可运行参照：

1. Core：添加 `NodeKind` 变体，选择 `ModuleFamily`。
2. Core：写 `ModuleDesc`，确保端口、默认参数、标签长度和真实算法一致，更新 `NodeKind::desc()` 下标。
3. DSP：在 `src/nodes/<family>/` 实现 `Process`，在该目录及 `nodes/mod.rs` 导出。
4. DSP：在 `for_kind` 绑定必需的参数单元、构造处理器；新增公开类型时在 `lib.rs` 导出。
5. UI：0..=1 参数可使用通用检查器；其他范围/枚举需要专用控件或元数据扩展。确认节点尺寸足够容纳所有端口。
6. 验证完整链路后将 `addable` 设为 true；目录可添加状态应与工厂可实例化状态一致。
7. 在伞仓运行测试、Clippy、rustdoc；分别提交子仓库，再更新伞仓 submodule 指针。

Noise 的 `with_params` 使用固定默认种子，适合可重复测试；`with_seed` 接受显式种子，零会替换为非零种子。工厂按 `NodeId + 1` 分配种子，使通常的不同实例不再逐样本相同（u32 极限 ID 的零种子回退并不保证全域唯一）。同一实例的流在图重建后从种子重新开始。它是普通合成器噪声源，不是密码学随机数发生器。

## 7. 验证清单

- 元数据：catalog 无重复、kind 映射正确、family 能在 `ALL` 找到、参数切片长度匹配。
- 工厂：所有可添加模块能实例化，计划项和缺失参数返回 `None`。
- DSP：已知输入输出、零参数、连续块、短块、有效范围和输出尾部不被改写。
- 参数：持有的 `Arc<ParamCell>` 更新后，下一块生效，不串改其他实例。
- 集成：新模块 → Output 能输出，检查器数值区域在窄窗口内可见。

```sh
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
RUSTDOCFLAGS='-D warnings' cargo doc --workspace --no-deps
```

本页 Rust 示例作为 `waver-dsp` 的 rustdoc 测试执行。测试通过不等于回调无分配，也不替代真实设备的长期运行测试。

# Hermes Control

Hermes Control 是一个面向 Windows + WSL2 的本地 AI 控制器。它的目标不是再做一个聊天客户端，而是接管本地 Hermes 运行栈的关键控制面：WSL2 生命周期、Hermes 进程、vLLM 本地模型运行时、Open WebUI 接入、路由切换、日志、健康检查和安全确认。

当前项目已完成 Phase 5 基础收尾，并开始 Phase 6：Rust 控制核心、CLI、daemon、bot 边界、WSL root helper、项目内 vLLM 运行时和 qwen36 MTP 实测链路已经建立；route switcher 已有状态切换骨架；GUI 和完整安装向导还在后续阶段。

## 当前能力

已经落地：

- Rust workspace 和 typed control core。
- Windows daemon API 边界。
- CLI thin client。
- Telegram bot thin client 边界。
- WSL/Hermes 固定 root helper 调用边界。
- 项目自有 vLLM 运行时目录：
  `E:\WSL\Hermres\hermes-control\vLLM`
- 外部模型权重目录：
  `E:\WSL\vLLM\models`
- vLLM bootstrap / start / stop / health / logs helper。
- MTP 启动失败时回退到 AWQ 稳定 profile 的 root helper 边界。
- `qwen36-mtp` MTP 模型实测启动和调用。
- Hermes gateway 调用本地模型实测。
- Open WebUI 经 Hermes gateway 调用本地模型实测。
- daemon/CLI 级 route active 与 route switch 状态骨架。

仍在规划或后续阶段：

- 完整 GUI。
- 一键 Fresh Install 向导。
- Hermes/Open WebUI 接入自动化。
- Hermes/Open WebUI route switcher 的配置补丁、热重载和回滚。
- benchmark 持久化和 GUI 展示。
- Windows service/installer。

## 目录结构

```text
hermes-control
├─ crates
│  ├─ hermes-control-core      # 状态采集、计划生成、typed operation core
│  ├─ hermes-control-daemon    # Windows 本地 HTTP API daemon
│  ├─ hermes-control-cli       # CLI thin client
│  ├─ hermes-control-bot       # Telegram bot thin client
│  ├─ hermes-control-gui       # 未来 GUI 边界 crate
│  ├─ hermes-control-testkit   # 测试辅助
│  └─ hermes-control-types     # API DTO 和共享类型
├─ config                      # daemon/provider/model runtime 配置
├─ docs                        # 变更记录、交接、代码地图、接入说明
├─ scripts/wsl-root            # 安装到 WSL /opt/hermes-control/bin 的 helper
├─ vLLM                        # 项目自有 vLLM runtime、脚本、venv/cache/logs
├─ plan_rust_control_rewrite.md
└─ plan_wsl2_hermes_provisioning.md
```

## 重要边界

`E:\WSL\Hermres\hermes-control\vLLM` 是软件自有的 vLLM 运行环境，里面可以放 venv、cache、logs、scripts。

`E:\WSL\vLLM\models` 只是模型权重仓库。安装、修复、清理流程默认不得删除这里的模型文件。

WSL 内 root helper 安装到：

```text
/opt/hermes-control/bin
```

运行时配置在：

```text
/etc/hermes-control/runtime.env
```

Hermes Control 不提供任意 shell 执行接口。所有危险操作必须是 Rust enum 到固定命令构造器，再通过 daemon 审计和确认。

## 开发环境

当前开发机使用 Windows PowerShell。

进入项目：

```powershell
cd E:\WSL\Hermres\hermes-control
```

常用验证：

```powershell
cargo fmt --all -- --check
cargo test --workspace
cargo build --workspace
cargo clippy --workspace --all-targets -- -D warnings
git diff --check
```

## WSL Root Helper 安装

刷新 WSL root helper：

```powershell
wsl.exe -d Ubuntu-Hermes-Codex -u root --exec bash -lc "cd /mnt/e/WSL/Hermres/hermes-control && bash scripts/wsl-root/install.sh"
```

检查 Hermes：

```powershell
wsl.exe -d Ubuntu-Hermes-Codex -u root --exec /opt/hermes-control/bin/hermes-control-status.sh
wsl.exe -d Ubuntu-Hermes-Codex -u root --exec /opt/hermes-control/bin/hermes-control-health.sh 30 ready
```

## vLLM 运行时

Bootstrap 或修复项目内 vLLM 环境：

```powershell
wsl.exe -d Ubuntu-Hermes-Codex -u root --exec /opt/hermes-control/bin/hermes-control-vllm-bootstrap.sh qwen36-mtp
```

启动 MTP 模型：

```powershell
wsl.exe -d Ubuntu-Hermes-Codex -u root --exec /opt/hermes-control/bin/hermes-control-vllm-start-with-fallback.sh qwen36-mtp qwen36-awq-int4
```

等待 ready：

```powershell
wsl.exe -d Ubuntu-Hermes-Codex -u root --exec /opt/hermes-control/bin/hermes-control-vllm-health.sh qwen36-mtp 600 ready
```

停止模型：

```powershell
wsl.exe -d Ubuntu-Hermes-Codex -u root --exec /opt/hermes-control/bin/hermes-control-vllm-stop.sh qwen36-mtp
```

注意：vLLM 在 WSL 上不应假设永远通过 `127.0.0.1:18080` 可调用。helper 会在 `VLLM_MODELS_ENDPOINT=auto` 时解析 WSL 主 IP，并输出实际 `models_endpoint`。

## Hermes 和 Open WebUI 接入

推荐链路：

```text
Open WebUI -> Hermes Gateway -> vLLM
```

这样 Hermes 保持 AI route owner，Open WebUI 不需要直接了解 vLLM 细节。

当前机器实测链路：

```text
vLLM endpoint:  http://10.2.176.55:18080/v1
Hermes health:  http://127.0.0.1:8642/health
Open WebUI:     http://127.0.0.1:3000
```

这个 IP 只是当前机器快照。新机器必须由 helper 动态解析。

完整 WSL2/Hermes/Open WebUI 接入规范见：

```text
plan_wsl2_hermes_provisioning.md
```

## Route Switch

当前 Phase 6 已经具备状态层 route switch：

```powershell
cargo run -p hermes-control-cli -- --api-token <token> route active
cargo run -p hermes-control-cli -- --api-token <token> route switch external.openai-compatible --dry-run --reason "smoke"
```

这一步会验证 provider、维护 active route 和 last-known-good route，并在切到 local vLLM profile 时检查模型 ready。它暂时还不会自动改 Hermes/Open WebUI 配置；这正是 Phase 6 下一步。

## 文档入口

- `plan_rust_control_rewrite.md`：Rust 控制器主计划。
- `plan_wsl2_hermes_provisioning.md`：WSL2、Hermes、Open WebUI、vLLM 部署规范。
- `docs/AI_CHANGE_GUIDE.md`：AI 修改规则。
- `docs/AI_HANDOFF.md`：当前交接状态。
- `docs/APP_CODE_MAP.md`：代码地图。
- `docs/RECENT_CHANGES.md`：最近变更记录。
- `docs/wsl-root-integration.md`：WSL root helper 接入说明。

## 安全原则

- GUI、bot、CLI 都是 daemon client，不直接控制系统。
- daemon 只接受 typed action。
- destructive action 必须 dry-run、确认、审计。
- 不打印 token、API key、JWT、Open WebUI secret。
- 不静默覆盖 Hermes/Open WebUI 配置。
- 不删除模型权重目录。

## Git 协作

当前约定：当我认为一组修改值得提交和推送时，会先给出建议 commit title，并等待批准后再执行 commit + push。

代理 push 示例：

```powershell
git -c http.proxy=http://127.0.0.1:7890 -c https.proxy=http://127.0.0.1:7890 push origin main
```

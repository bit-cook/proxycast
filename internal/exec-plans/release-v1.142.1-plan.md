# Lime v1.142.1 发布执行计划

状态：准备中
日期：2026-09-09
目标版本：`1.142.1`
目标 tag：`v1.142.1`

## 主目标

在不改写已公开的 `v1.142.0` tag 的前提下，修复 Release workflow 的 Windows npm 平台包打包失败、
Linux CLI runner 缺少 ALSA 开发依赖，以及同一轮质量检查发现的 command 取消结果投影和回归断言问题；
重新生成根 npm、Linux x64、Windows x64 资产并完成补丁版发布。

## Release Candidate

- `release metadata`：`package.json`、`packages/cli/package.json`、`lime-rs/Cargo.toml`、
  `lime-rs/Cargo.lock`、`RELEASE_NOTES.md`、`RELEASE_NOTES.en.md`、本计划和
  `internal/exec-plans/README.md`。
- `candidate changes`：`.github/workflows/release.yml`、`packages/cli/scripts/build_npm_package.py`、
  `electron/hostCommands.test.ts`、App Server command event mirror 与其测试。
- `excluded changes`：未跟踪的本机 Mach-O `rust_out`；`.lime/`、`dist/`、`dist-electron/` 和
  `lime-rs/target/` 等被忽略的本地生成目录。

## 架构确认

本补丁不改变业务主链，继续遵循：

```text
Electron Desktop Host / CLI-TUI Host
  -> App Server JSON-RPC
  -> RuntimeCore
  -> Thread / Turn / Item projection
```

修改集中在发布脚本/runner 依赖和既有 command projection 测试行为，不新增 owner、协议或 runtime。

## 退出条件

- 所有版本事实源和双语单页 release notes 统一为 `1.142.1`。
- `npm run verify:app-version`、`npm run typecheck`、`npm run test:contracts`、CLI npm 定向测试、
  脚本/Electron release governance、`cargo fmt --manifest-path "lime-rs/Cargo.toml" --all -- --check`
  和 `git diff --check` 通过。
- Rust 行为测试若继续受本机 `rusty_v8` archive 404 阻塞，必须在收尾报告明确标注，并依赖 CI runner
  结果确认。
- 获得新的危险操作确认后，创建 `Release v1.142.1` commit、`v1.142.1` tag，推送 `origin/main`
  与远端 tag，并确认 Release 资产完整。

## 已知阻塞

- 本机 Darwin/aarch64 无法下载 `rusty_v8 v150.4.0` 预构建 archive；不把该环境限制误报为代码失败。
- Windows 真机/打包矩阵、签名/公证和完整 Forge 产物必须由 GitHub runner 提供。

## 收尾分类

- `current`：跨平台 npm 打包入口、Release runner 依赖、command completion result 投影。
- `compat`：无新增。
- `deprecated`：无新增。
- `dead / deleted`：无新增。

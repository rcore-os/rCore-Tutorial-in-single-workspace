# 第一章实验

第一章实验的示例，展示如何依赖 `rcore_console` crate。

在 [Cargo.toml](Cargo.toml#L9) 里添加：

```toml
rcore_console = { path = "../rcore_console"}
```

在 [main.rs](src/main.rs#L38) 里初始化：

```rust
rcore_console::init_console(&Console);
```

后续的章节都可以这样依赖 `rcore_console`。

<a id="source-nav"></a>

## 源码阅读导航索引

[返回根文档导航总表](../README.md#chapters-source-nav-map)

本实验是 `ch1` 的补充，建议按下面顺序阅读并动手验证。

| 阅读顺序 | 文件 | 重点问题 |
|---|---|---|
| 1 | `src/main.rs` 的 `_start` | 裸机入口如何手动设置栈并跳转到 Rust 代码？ |
| 2 | `src/main.rs` 的 `rust_main` | 控制台初始化后，`print!/println!` 为何就能工作？ |
| 3 | `src/main.rs` 的 `Console` 实现 | `put_char` 如何打通到 SBI 输出路径？ |
| 4 | `src/main.rs` 的 `panic_handler` | no_std 环境发生 panic 后为什么要主动关机？ |

配套建议：与 `ch1/src/main.rs` 对照阅读，可以快速看出“最小输出程序”和“带 console 抽象程序”的差异。

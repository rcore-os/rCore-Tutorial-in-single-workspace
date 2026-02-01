# tg-user

本 crate 提供 rCore Tutorial 用户态程序（当前包含与 ch2 对应的用户程序集合）。

## 内容

- `cases.toml`：定义要编译并打包的用户程序集合
- `src/lib.rs`：用户态运行时入口与基础工具
- `src/bin/*`：各用户程序

## 用途

该 crate 设计为被 tg-ch2 等内核章节在构建阶段拉取并编译用户程序。

## License

Licensed under either of MIT license or Apache License, Version 2.0 at your option.

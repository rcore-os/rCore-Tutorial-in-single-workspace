# tg-checker

rCore-Tutorial 测试输出检测工具。

## 设计目标

- 将章节运行输出自动化比对，减少人工逐行核对成本。
- 为教学实验提供统一、可复现的通过/失败判断标准。
- 支持基础题与 exercise 模式的分离校验。

## 总体架构

- `src/checker.rs`：核心匹配逻辑（期望/非期望模式校验）。
- `src/cases/`：按章节组织测试规则（`ch2` 到 `ch8`）。
- `src/main.rs`：CLI 参数解析与结果输出。

## 主要特征

- 支持基础模式与 exercise 模式。
- 支持列出可用测试集。
- 输出包含失败项细节，便于快速定位问题。
- 以正则规则驱动测试匹配。

## 功能实现要点

- 先选择章节测试用例，再对运行输出做规则匹配。
- 同时处理“必须出现”和“禁止出现”两类规则。
- 提供详细失败信息，便于对照章节实验手册修复。

## 对外接口

- 命令行：
  - `--ch <N>`：章节号（2-8）
  - `--exercise`：exercise 测试
  - `--list`：列出可用测试
- 作为库（可选）：
  - `check(output, test_case)`
  - `print_result(result, verbose)`
  - `cases::get_test_case(...)`

## 使用示例

安装：

```bash
cargo install tg-checker
```

基础测试（ch2-ch8）：

```bash
cargo run 2>&1 | tg-checker --ch 2
```

Exercise 测试（ch3/ch4/ch5/ch6/ch8）：

```bash
cargo run --features exercise 2>&1 | tg-checker --ch 3 --exercise
```

## 与 ch1~ch8 的关系

- 直接依赖章节：无（不是章节内核的 Cargo 运行依赖）。
- 关键职责：作为测试链路工具校验 `ch2~ch8` 的运行输出。
- 关键引用文件：
  - `ch2/test.sh`
  - `ch3/test.sh`
  - `ch8/test.sh`
  - `tg-checker/src/cases/ch2.rs` 到 `tg-checker/src/cases/ch8.rs`

## License

MIT OR Apache-2.0

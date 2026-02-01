# tg-checker

rCore-Tutorial 测试输出检测工具。

## 安装

从本地安装:
```
cargo install tg-checker
```

## 使用方法

基础测试 (ch2-ch8):
```
cargo run 2>&1 | tg-checker --ch 2
```

Exercise 测试 (ch3, ch4, ch5, ch6, ch8):
```
cargo run --features exercise 2>&1 | tg-checker --ch 3 --exercise
```

## 命令行选项

- `--ch <N>` 章节号 (2-8)
- `--exercise` Exercise 模式
- `--list` 列出所有可用测试

## License

MIT OR Apache-2.0

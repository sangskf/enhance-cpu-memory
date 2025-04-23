#一个简易的CPU负载工具

通过复杂的数学计算来提高CPU利用率。

## 使用方法

```text
Usage: EhanceCPU [OPTIONS] [COMMAND]

Commands:
  status  查看当前CPU使用率
  start   启动CPU负载
  stop    停止正在运行的CPU负载
  help    Print this message or the help of the given subcommand(s)

Options:
  -c, --cores <CORES>  要使用的CPU核心数量，默认为系统核心数的一半（至少为1） [default: 4]
  -b, --background     是否在后台运行
  -h, --help           Print help
  -V, --version        Print version

```

## 版本更新
- 1.0.0 初始版本，支持基本的CPU负载功能。
- 1.0.1 修复了一些bug。
- 1.0.2 修复了一些bug。
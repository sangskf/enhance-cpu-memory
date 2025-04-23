# 系统负载工具 (enhance-cpu-memory)

一个用于测试和模拟系统负载的命令行工具，可以精确控制CPU和内存的使用率。

## 功能特点

- 精确控制CPU核心使用数量
- 可配置内存占用大小
- 支持后台运行模式
- 实时监控系统资源使用情况
- 支持优雅停止

## 使用方法

### 基本命令

```bash
enhance-cpu-memory [OPTIONS] [COMMAND]
```

### 命令说明

- `status`: 查看当前系统CPU和内存使用状态
- `start`: 启动系统负载
- `stop`: 停止正在运行的负载

### 参数选项

- `-c, --cores <数量>`: 指定要使用的CPU核心数
  - 默认值：系统核心数的一半（至少为1）
  - 示例：`enhance-cpu-memory -c 4`（使用4个核心）

- `-m, --memory <大小>`: 指定要占用的内存大小
  - 支持的单位：B, K, M, G, T
  - 示例：`enhance-cpu-memory -m 1G`（占用1GB内存）

- `-b, --background`: 在后台运行
  - 示例：`enhance-cpu-memory -b`

### 使用示例

1. 查看系统状态：
```bash
enhance-cpu-memory status
```

2. 启动负载（使用2个核心）：
```bash
enhance-cpu-memory start -c 2
```

3. 启动负载并占用内存(同时占用CPU和内存)：
```bash
enhance-cpu-memory start -c 2 -m 512M
```

4. 后台运行负载(同时占用CPU和内存)：
```bash
enhance-cpu-memory start -c 4 -m 1G -b
```

5. 直接启动（使用默认参数，默认只占用CPU核心的一半）：
```bash
enhance-cpu-memory
```

6. 停止负载：
```bash
enhance-cpu-memory stop
```

## 注意事项

1. 内存参数支持的单位：
   - B：字节
   - K：千字节（KB）
   - M：兆字节（MB）
   - G：千兆字节（GB）
   - T：太字节（TB）

2. CPU核心数不能超过系统实际核心数
3. 后台运行时，请使用`stop`命令来停止负载
4. 使用Ctrl+C可以优雅地停止前台运行的负载

## 版本更新

- 1.0.0：初始版本，支持基本的CPU负载功能
- 1.0.1：添加内存占用功能
- 1.0.2：添加后台运行模式
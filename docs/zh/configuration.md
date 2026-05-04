# RorisDB 配置说明

本文档详细说明 RorisDB 的配置选项，包括 FE 和 BE 的配置参数。

## 配置文件位置

### FE 配置文件
- 默认路径：`./conf/fe.conf`
- 或通过命令行参数 `--config` 指定

### BE 配置文件
- 默认路径：`./conf/be.conf`
- 或通过命令行参数 `--config` 指定

## FE 配置参数

### 网络配置

| 参数 | 说明 | 默认值 | 示例 |
|------|------|--------|------|
| `http_port` | HTTP 服务端口（用于 Web UI 和 API） | `8030` | `http_port = 8030` |
| `rpc_port` | gRPC 服务端口（与 BE 通信） | `9020` | `rpc_port = 9020` |
| `mysql_port` | MySQL 协议端口（客户端连接） | `9030` | `mysql_port = 9030` |
| `bind_address` | 绑定的 IP 地址 | `0.0.0.0` | `bind_address = 0.0.0.0` |

### 存储配置

| 参数 | 说明 | 默认值 | 示例 |
|------|------|--------|------|
| `data_dir` | 元数据存储目录 | `./fe_data` | `data_dir = /data/roris/fe` |
| `edit_log_dir` | EditLog 存储目录 | `<data_dir>/edit_log` | `edit_log_dir = /data/roris/fe/edit_log` |

### 日志配置

| 参数 | 说明 | 默认值 | 示例 |
|------|------|--------|------|
| `log_level` | 日志级别（trace, debug, info, warn, error） | `info` | `log_level = debug` |
| `log_dir` | 日志文件目录 | `./logs` | `log_dir = /var/log/roris` |
| `log_to_file` | 是否输出到文件 | `true` | `log_to_file = true` |
| `log_to_stderr` | 是否输出到标准错误 | `false` | `log_to_stderr = true` |

### 集群配置

| 参数 | 说明 | 默认值 | 示例 |
|------|------|--------|------|
| `cluster_name` | 集群名称 | `rorisdb` | `cluster_name = production` |
| `ha_mode` | 高可用模式（当前为单点，规划中支持 Raft） | `standalone` | `ha_mode = raft` |

### 查询配置

| 参数 | 说明 | 默认值 | 示例 |
|------|------|--------|------|
| `query_timeout` | 查询超时时间（秒） | `300` | `query_timeout = 600` |
| `max_concurrent_queries` | 最大并发查询数 | `100` | `max_concurrent_queries = 200` |

### 示例 FE 配置文件

```ini
# fe.conf
http_port = 8030
rpc_port = 9020
mysql_port = 9030
bind_address = 0.0.0.0

data_dir = /data/roris/fe
edit_log_dir = /data/roris/fe/edit_log

log_level = info
log_dir = /var/log/roris/fe
log_to_file = true
log_to_stderr = false

cluster_name = rorisdb
ha_mode = standalone

query_timeout = 300
max_concurrent_queries = 100
```

## BE 配置参数

### 网络配置

| 参数 | 说明 | 默认值 | 示例 |
|------|------|--------|------|
| `http_port` | HTTP 服务端口（用于监控和健康检查） | `8060` | `http_port = 8060` |
| `rpc_port` | gRPC 服务端口（与 FE 通信） | `9060` | `rpc_port = 9060` |
| `bind_address` | 绑定的 IP 地址 | `0.0.0.0` | `bind_address = 0.0.0.0` |

### FE 连接配置

| 参数 | 说明 | 默认值 | 示例 |
|------|------|--------|------|
| `fe_addr` | FE 的 gRPC 地址（用于注册） | `127.0.0.1:9020` | `fe_addr = 192.168.1.10:9020` |

### 存储配置

| 参数 | 说明 | 默认值 | 示例 |
|------|------|--------|------|
| `data_dir` | 数据存储根目录 | `./be_data` | `data_dir = /data/roris/be` |
| `storage_root_path` | 存储路径（支持多块盘，逗号分隔） | `<data_dir>/storage` | `storage_root_path = /data1/roris,/data2/roris` |
| `tablet_path_prefix` | Tablet 目录前缀 | `tablet` | `tablet_path_prefix = tablet` |

### 内存配置

| 参数 | 说明 | 默认值 | 示例 |
|------|------|--------|------|
| `memory_limit` | 内存限制（支持 GB、MB 单位） | `8GB` | `memory_limit = 16GB` |
| `memtable_size` | MemTable 大小（触发 flush） | `128MB` | `memtable_size = 256MB` |

### 日志配置

| 参数 | 说明 | 默认值 | 示例 |
|------|------|--------|------|
| `log_level` | 日志级别 | `info` | `log_level = debug` |
| `log_dir` | 日志文件目录 | `./logs` | `log_dir = /var/log/roris/be` |
| `log_to_file` | 是否输出到文件 | `true` | `log_to_file = true` |
| `log_to_stderr` | 是否输出到标准错误 | `false` | `log_to_stderr = true` |

### Compaction 配置

| 参数 | 说明 | 默认值 | 示例 |
|------|------|--------|------|
| `cumulative_compaction_min_rowset` | Cumulative Compaction 最小 Rowset 数 | `5` | `cumulative_compaction_min_rowset = 10` |
| `cumulative_compaction_max_rowset` | Cumulative Compaction 最大 Rowset 数 | `20` | `cumulative_compaction_max_rowset = 30` |
| `base_compaction_interval_secs` | Base Compaction 检查间隔（秒） | `3600` | `base_compaction_interval_secs = 7200` |

### 查询执行配置

| 参数 | 说明 | 默认值 | 示例 |
|------|------|--------|------|
| `batch_size` | 向量化执行批次大小 | `1024` | `batch_size = 2048` |
| `max_threads` | 执行最大线程数（0 表示使用 CPU 核心数） | `0` | `max_threads = 8` |

### 示例 BE 配置文件

```ini
# be.conf
http_port = 8060
rpc_port = 9060
bind_address = 0.0.0.0

fe_addr = 127.0.0.1:9020

data_dir = /data/roris/be
storage_root_path = /data/roris/be/storage
tablet_path_prefix = tablet

memory_limit = 16GB
memtable_size = 256MB

log_level = info
log_dir = /var/log/roris/be
log_to_file = true
log_to_stderr = false

cumulative_compaction_min_rowset = 5
cumulative_compaction_max_rowset = 20
base_compaction_interval_secs = 3600

batch_size = 1024
max_threads = 0
```

## 命令行参数

除了配置文件，也可以通过命令行参数传递配置。命令行参数优先级高于配置文件。

### FE 命令行参数

```bash
./target/release/roris-fe --help
```

常用参数：
```bash
./target/release/roris-fe \
  --http-port 8030 \
  --rpc-port 9020 \
  --mysql-port 9030 \
  --data-dir /data/roris/fe \
  --log-level info \
  --config conf/fe.conf
```

### BE 命令行参数

```bash
./target/release/roris-be --help
```

常用参数：
```bash
./target/release/roris-be \
  --http-port 8060 \
  --rpc-port 9060 \
  --fe-addr 127.0.0.1:9020 \
  --data-dir /data/roris/be \
  --memory-limit 16GB \
  --config conf/be.conf
```

## 环境变量

### Rust 相关环境变量

| 变量 | 说明 | 示例 |
|------|------|------|
| `RUST_LOG` | Rust 日志级别（覆盖配置文件） | `RUST_LOG=debug` |
| `RUST_BACKTRACE` | 启用 backtrace | `RUST_BACKTRACE=1` |
| `RAYON_NUM_THREADS` | 限制 Rayon 线程数 | `RAYON_NUM_THREADS=8` |

示例：
```bash
export RUST_LOG=debug
export RAYON_NUM_THREADS=8
./target/release/roris-be ...
```

## 配置优先级

配置加载优先级（从高到低）：

1. 命令行参数
2. 环境变量
3. 配置文件
4. 默认值

## 配置最佳实践

### 生产环境建议

1. **使用专用用户运行**
   ```bash
   useradd -r -s /bin/false roris
   chown -R roris:roris /data/roris
   ```

2. **分离日志和数据目录**
   ```ini
   data_dir = /data/roris/fe
   log_dir = /var/log/roris
   ```

3. **合理配置内存**
   ```ini
   # BE 内存限制（建议为物理内存的 70-80%）
   memory_limit = 16GB
   ```

4. **使用 SSD 存储**
   ```ini
   storage_root_path = /ssd1/roris,/ssd2/roris
   ```

5. **配置日志轮转**
   使用 logrotate 或其他工具管理日志文件

### 开发环境建议

```ini
# 更详细的日志
log_level = debug
log_to_stderr = true

# 较小的数据批次（方便调试）
batch_size = 256
```

## 常见问题

### 端口被占用

**问题**：启动时提示端口已被占用

**解决**：
1. 修改配置文件中的端口号
2. 或检查是否有其他进程占用该端口：
   ```bash
   lsof -i :9030
   ```

### 内存不足

**问题**：查询或导入时出现内存不足错误

**解决**：
1. 增加 BE 的 `memory_limit`
2. 减小 `batch_size`
3. 增加系统 swap 空间

### 日志文件过大

**问题**：日志文件占用过多磁盘空间

**解决**：
1. 降低 `log_level` 为 `warn` 或 `error`
2. 配置日志轮转
3. 定期清理旧日志

## 下一步

- 查看[安装部署指南](installation.md)了解如何启动服务
- 阅读[快速开始](getting-started.md)开始使用 RorisDB
- 参考[SQL 参考手册](sql-reference.md)学习 SQL 语法

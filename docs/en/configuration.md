# RorisDB Configuration Guide

This document details RorisDB's configuration options, including parameters for FE (Frontend) and BE (Backend).

## Configuration File Locations

### FE Configuration File
- Default path: `./conf/fe.conf`
- Or specify via the `--config` command-line argument

### BE Configuration File
- Default path: `./conf/be.conf`
- Or specify via the `--config` command-line argument

## FE Configuration Parameters

### Network Configuration

| Parameter | Description | Default Value | Example |
|-----------|-------------|---------------|---------|
| `http_port` | HTTP service port (for Web UI and API) | `8030` | `http_port = 8030` |
| `rpc_port` | gRPC service port (for communication with BE) | `9020` | `rpc_port = 9020` |
| `mysql_port` | MySQL protocol port (for client connections) | `9030` | `mysql_port = 9030` |
| `bind_address` | Bound IP address | `0.0.0.0` | `bind_address = 0.0.0.0` |

### Storage Configuration

| Parameter | Description | Default Value | Example |
|-----------|-------------|---------------|---------|
| `data_dir` | Metadata storage directory | `./fe_data` | `data_dir = /data/roris/fe` |
| `edit_log_dir` | EditLog storage directory | `<data_dir>/edit_log` | `edit_log_dir = /data/roris/fe/edit_log` |

### Logging Configuration

| Parameter | Description | Default Value | Example |
|-----------|-------------|---------------|---------|
| `log_level` | Log level (trace, debug, info, warn, error) | `info` | `log_level = debug` |
| `log_dir` | Log file directory | `./logs` | `log_dir = /var/log/roris` |
| `log_to_file` | Whether to output logs to file | `true` | `log_to_file = true` |
| `log_to_stderr` | Whether to output logs to stderr | `false` | `log_to_stderr = true` |

### Cluster Configuration

| Parameter | Description | Default Value | Example |
|-----------|-------------|---------------|---------|
| `cluster_name` | Cluster name | `rorisdb` | `cluster_name = production` |
| `ha_mode` | High availability mode (currently standalone, Raft support planned) | `standalone` | `ha_mode = raft` |

### Query Configuration

| Parameter | Description | Default Value | Example |
|-----------|-------------|---------------|---------|
| `query_timeout` | Query timeout (seconds) | `300` | `query_timeout = 600` |
| `max_concurrent_queries` | Maximum number of concurrent queries | `100` | `max_concurrent_queries = 200` |

### Sample FE Configuration File

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

## BE Configuration Parameters

### Network Configuration

| Parameter | Description | Default Value | Example |
|-----------|-------------|---------------|---------|
| `http_port` | HTTP service port (for monitoring and health checks) | `8060` | `http_port = 8060` |
| `rpc_port` | gRPC service port (for communication with FE) | `9060` | `rpc_port = 9060` |
| `bind_address` | Bound IP address | `0.0.0.0` | `bind_address = 0.0.0.0` |

### FE Connection Configuration

| Parameter | Description | Default Value | Example |
|-----------|-------------|---------------|---------|
| `fe_addr` | gRPC address of FE (for registration) | `127.0.0.1:9020` | `fe_addr = 192.168.1.10:9020` |

### Storage Configuration

| Parameter | Description | Default Value | Example |
|-----------|-------------|---------------|---------|
| `data_dir` | Root directory for data storage | `./be_data` | `data_dir = /data/roris/be` |
| `storage_root_path` | Storage paths (supports multiple disks, comma-separated) | `<data_dir>/storage` | `storage_root_path = /data1/roris,/data2/roris` |
| `tablet_path_prefix` | Prefix for tablet directories | `tablet` | `tablet_path_prefix = tablet` |

### Memory Configuration

| Parameter | Description | Default Value | Example |
|-----------|-------------|---------------|---------|
| `memory_limit` | Memory limit (supports GB, MB units) | `8GB` | `memory_limit = 16GB` |
| `memtable_size` | MemTable size (triggers flush when reached) | `128MB` | `memtable_size = 256MB` |

### Logging Configuration

| Parameter | Description | Default Value | Example |
|-----------|-------------|---------------|---------|
| `log_level` | Log level | `info` | `log_level = debug` |
| `log_dir` | Log file directory | `./logs` | `log_dir = /var/log/roris/be` |
| `log_to_file` | Whether to output logs to file | `true` | `log_to_file = true` |
| `log_to_stderr` | Whether to output logs to stderr | `false` | `log_to_stderr = true` |

### Compaction Configuration

| Parameter | Description | Default Value | Example |
|-----------|-------------|---------------|---------|
| `cumulative_compaction_min_rowset` | Minimum number of Rowsets for Cumulative Compaction | `5` | `cumulative_compaction_min_rowset = 10` |
| `cumulative_compaction_max_rowset` | Maximum number of Rowsets for Cumulative Compaction | `20` | `cumulative_compaction_max_rowset = 30` |
| `base_compaction_interval_secs` | Base Compaction check interval (seconds) | `3600` | `base_compaction_interval_secs = 7200` |

### Query Execution Configuration

| Parameter | Description | Default Value | Example |
|-----------|-------------|---------------|---------|
| `batch_size` | Batch size for vectorized execution | `1024` | `batch_size = 2048` |
| `max_threads` | Maximum number of execution threads (0 means use number of CPU cores) | `0` | `max_threads = 8` |

### Sample BE Configuration File

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

## Command-Line Arguments

In addition to configuration files, parameters can also be passed via command-line arguments. Command-line arguments take precedence over configuration files.

### FE Command-Line Arguments

```bash
./target/release/roris-fe --help
```

Common parameters:
```bash
./target/release/roris-fe \
  --http-port 8030 \
  --rpc-port 9020 \
  --mysql-port 9030 \
  --data-dir /data/roris/fe \
  --log-level info \
  --config conf/fe.conf
```

### BE Command-Line Arguments

```bash
./target/release/roris-be --help
```

Common parameters:
```bash
./target/release/roris-be \
  --http-port 8060 \
  --rpc-port 9060 \
  --fe-addr 127.0.0.1:9020 \
  --data-dir /data/roris/be \
  --memory-limit 16GB \
  --config conf/be.conf
```

## Environment Variables

### Rust-Related Environment Variables

| Variable | Description | Example |
|----------|-------------|---------|
| `RUST_LOG` | Rust log level (overrides configuration file) | `RUST_LOG=debug` |
| `RUST_BACKTRACE` | Enable backtrace | `RUST_BACKTRACE=1` |
| `RAYON_NUM_THREADS` | Limit number of Rayon threads | `RAYON_NUM_THREADS=8` |

Example:
```bash
export RUST_LOG=debug
export RAYON_NUM_THREADS=8
./target/release/roris-be ...
```

## Configuration Priority

Configuration loading priority (from highest to lowest):

1. Command-line arguments
2. Environment variables
3. Configuration files
4. Default values

## Configuration Best Practices

### Production Environment Recommendations

1. **Run with a dedicated user**
   ```bash
   useradd -r -s /bin/false roris
   chown -R roris:roris /data/roris
   ```

2. **Separate log and data directories**
   ```ini
   data_dir = /data/roris/fe
   log_dir = /var/log/roris
   ```

3. **Configure memory appropriately**
   ```ini
   # BE memory limit (recommended 70-80% of physical memory)
   memory_limit = 16GB
   ```

4. **Use SSD storage**
   ```ini
   storage_root_path = /ssd1/roris,/ssd2/roris
   ```

5. **Configure log rotation**
   Use logrotate or other tools to manage log files.

### Development Environment Recommendations

```ini
# More detailed logging
log_level = debug
log_to_stderr = true

# Smaller batch sizes (for easier debugging)
batch_size = 256
```

## Frequently Asked Questions

### Port Already in Use

**Problem**: Port already in use error when starting the service.

**Solution**:
1. Modify the port number in the configuration file.
2. Or check if another process is using the port:
   ```bash
   lsof -i :9030
   ```

### Out of Memory

**Problem**: Out of memory error during queries or data imports.

**Solution**:
1. Increase BE's `memory_limit`.
2. Reduce `batch_size`.
3. Increase system swap space.

### Log Files Too Large

**Problem**: Log files take up too much disk space.

**Solution**:
1. Lower `log_level` to `warn` or `error`.
2. Configure log rotation.
3. Regularly clean up old logs.

## Next Steps

- See the [Installation Guide](installation.md) to learn how to start the service.
- Read the [Getting Started](getting-started.md) guide to start using RorisDB.
- Refer to the [SQL Reference Manual](sql-reference.md) to learn SQL syntax.

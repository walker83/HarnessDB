# RorisDB Installation Guide

## System Requirements

### Hardware Requirements
- **CPU**: x86_64 architecture, recommended 8 cores or more
- **Memory**: Recommended 16GB or more (depending on data scale)
- **Disk**: SSD recommended, at least 50GB available space
- **Network**: Gigabit Ethernet (for distributed deployment)

### Software Requirements
- **Operating System**: Linux (recommended) / macOS
- **Rust**: Version 1.75+
- **Cargo**: Matching Rust version
- **MySQL Client** (optional): For connecting to the database

### Dependencies
```bash
# Check Rust version
rustc --version

# If Rust is not installed, use rustup to install
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

## Building from Source

### 1. Get the Source Code

```bash
git clone https://github.com/your-repo/RorisDB.git
cd RorisDB
```

### 2. Build the Project

#### Development Mode (Fast compilation, less optimization)
```bash
cargo build
```

#### Release Mode (Optimized compilation, recommended for production)
```bash
cargo build --release
```

After compilation, the binaries are located at:
- `target/release/roris-fe`: Frontend server
- `target/release/roris-be`: Backend server
- `target/release/roris-cli`: Command-line client

### 3. Verify the Build

```bash
# Check version information
./target/release/roris-fe --version
./target/release/roris-be --version
```

## Single-Node Deployment

### Quick Start (Standalone Mode)

#### 1. Start FE (Frontend)

```bash
./target/release/roris-fe --http-port 8030 --rpc-port 9020
```

**Parameter Description**:
- `--http-port`: FE HTTP service port (default 8030)
- `--rpc-port`: FE gRPC service port (default 9020)
- `--mysql-port`: MySQL protocol port (default 9030)
- `--data-dir`: Metadata storage directory (default `./fe_data`)

#### 2. Start BE (Backend)

```bash
./target/release/roris-be --http-port 8060 --rpc-port 9060 --fe-addr 127.0.0.1:9020
```

**Parameter Description**:
- `--http-port`: BE HTTP service port (default 8060)
- `--rpc-port`: BE gRPC service port (default 9060)
- `--fe-addr`: FE gRPC address (for registration)
- `--data-dir`: Data storage directory (default `./be_data`)

#### 3. Connect to the Database

Using MySQL client:

```bash
mysql -h 127.0.0.1 -P 9030 -uroot
```

Or using the built-in CLI tool:

```bash
./target/release/roris-cli
```

### Verify the Deployment

Execute the following SQL after connecting:

```sql
-- Check version
SELECT version();

-- Create test database
CREATE DATABASE IF NOT EXISTS test;
USE test;

-- Create test table
CREATE TABLE user (
    id BIGINT PRIMARY KEY,
    name VARCHAR(64),
    age INT
) DUPLICATE KEY;

-- Insert data
INSERT INTO user VALUES (1, 'Alice', 30), (2, 'Bob', 25);

-- Query data
SELECT * FROM user WHERE age > 20;
```

## Distributed Deployment

### Architecture Overview

Distributed deployment requires:
- 1 FE node (or 3 for high availability, planned)
- Multiple BE nodes (at least 2 recommended)

```
┌──────────┐
│   FE     │
│ 8030/9020│
└────┬─────┘
     │
     ├──────────────┬──────────────┐
     ▼              ▼              ▼
┌─────────┐   ┌─────────┐   ┌─────────┐
│  BE 1   │   │  BE 2   │   │  BE 3   │
│ 9060    │   │ 9060    │   │ 9060    │
└─────────┘   └─────────┘   └─────────┘
```

### Deployment Steps

#### 1. Start FE on the FE Node

```bash
# FE node (192.168.1.10)
./target/release/roris-fe \
  --http-port 8030 \
  --rpc-port 9020 \
  --mysql-port 9030 \
  --data-dir /data/roris/fe
```

#### 2. Start BE on Each BE Node

```bash
# BE node 1 (192.168.1.11)
./target/release/roris-be \
  --http-port 8060 \
  --rpc-port 9060 \
  --fe-addr 192.168.1.10:9020 \
  --data-dir /data/roris/be

# BE node 2 (192.168.1.12)
./target/release/roris-be \
  --http-port 8060 \
  --rpc-port 9060 \
  --fe-addr 192.168.1.10:9020 \
  --data-dir /data/roris/be

# BE node 3 (192.168.1.13)
./target/release/roris-be \
  --http-port 8060 \
  --rpc-port 9060 \
  --fe-addr 192.168.1.10:9020 \
  --data-dir /data/roris/be
```

#### 3. Verify Cluster Status

Connect to FE:

```bash
mysql -h 192.168.1.10 -P 9030 -uroot
```

Check BE node status:

```sql
-- View cluster information (specific commands may vary based on implementation)
SHOW BACKENDS;
```

## Configuration Files

RorisDB supports configuration via configuration files or command-line parameters.

### FE Configuration File (`conf/fe.conf`)

```ini
# FE configuration example
http_port = 8030
rpc_port = 9020
mysql_port = 9030
data_dir = ./fe_data

# Log configuration
log_level = info
log_dir = ./logs

# Cluster configuration
# Currently single FE, high availability configuration planned
```

### BE Configuration File (`conf/be.conf`)

```ini
# BE configuration example
http_port = 8060
rpc_port = 9060
fe_addr = 127.0.0.1:9020
data_dir = ./be_data

# Storage configuration
storage_root_path = /data/roris/storage

# Log configuration
log_level = info
log_dir = ./logs

# Resource limits
memory_limit = 8GB
```

### Starting with Configuration Files

```bash
# FE with configuration file
./target/release/roris-fe --config conf/fe.conf

# BE with configuration file
./target/release/roris-be --config conf/be.conf
```

## Docker Deployment (Optional)

### Building Docker Image

Create `Dockerfile`:

```dockerfile
FROM rust:1.75 AS builder
WORKDIR /app
COPY . .
RUN cargo build --release

FROM debian:bullseye-slim
WORKDIR /app
COPY --from=builder /app/target/release/roris-fe .
COPY --from=builder /app/target/release/roris-be .
COPY --from=builder /app/target/release/roris-cli .
CMD ["./roris-fe"]
```

Build the image:

```bash
docker build -t rorisdb:latest .
```

### Using Docker Compose

Create `docker-compose.yml`:

```yaml
version: '3'
services:
  fe:
    image: rorisdb:latest
    command: ./roris-fe --http-port 8030 --rpc-port 9020
    ports:
      - "8030:8030"
      - "9020:9020"
      - "9030:9030"
    volumes:
      - ./fe_data:/app/fe_data

  be1:
    image: rorisdb:latest
    command: ./roris-be --http-port 8060 --rpc-port 9060 --fe-addr fe:9020
    depends_on:
      - fe
    volumes:
      - ./be1_data:/app/be_data

  be2:
    image: rorisdb:latest
    command: ./roris-be --http-port 8061 --rpc-port 9061 --fe-addr fe:9020
    depends_on:
      - fe
    volumes:
      - ./be2_data:/app/be_data
```

Start:

```bash
docker-compose up -d
```

## Performance Tuning

### Memory Configuration

BE memory usage mainly includes:
- MemTable (write buffer)
- Query execution memory
- Cache (planned)

Recommended configuration:
```bash
# BE startup parameters (example)
--memory-limit 16GB
```

### Concurrency Configuration

Rust programs use all CPU cores by default. You can limit this with environment variables:

```bash
# Limit the number of CPU cores used
export RAYON_NUM_THREADS=8
./target/release/roris-be ...
```

### Storage Optimization

- Use SSD for data directory storage
- Use separate disks for data and logs
- Monitor disk space regularly

## Monitoring and Operations

### Viewing Logs

```bash
# FE logs
tail -f ./fe_data/logs/roris-fe.log

# BE logs
tail -f ./be_data/logs/roris-be.log
```

### HTTP Interfaces

Both FE and BE provide HTTP interfaces for monitoring:

```bash
# FE status
curl http://127.0.0.1:8030/status

# BE status
curl http://127.0.0.1:8060/status
```

### Health Checks

```bash
# FE health check
curl http://127.0.0.1:8030/health

# BE health check
curl http://127.0.0.1:8060/health
```

## Common Issues

### Build Failure

**Issue**: `cargo build` fails with dependency errors

**Solution**:
```bash
# Update Rust toolchain
rustup update

# Clean and rebuild
cargo clean
cargo build --release
```

### Port Already in Use

**Issue**: Port already in use error when starting

**Solution**:
```bash
# Check port usage
lsof -i :9030

# Start with a different port
./target/release/roris-fe --mysql-port 9031
```

### BE Cannot Connect to FE

**Issue**: BE fails to register with FE after startup

**Solution**:
1. Check if FE is running properly
2. Check if the `--fe-addr` parameter is correct
3. Check if the firewall is blocking the port
4. Check BE logs for detailed errors

### Slow Query Performance

**Optimization Suggestions**:
1. Ensure using release mode compilation (`--release`)
2. Check if appropriate indexes are used
3. Use `EXPLAIN` to view the query plan
4. Consider adding more BE nodes for horizontal scaling

## Uninstallation

After stopping all services, simply delete the data directories:

```bash
# Stop services (Ctrl+C or use kill)
pkill roris-fe
pkill roris-be

# Delete data directories
rm -rf ./fe_data ./be_data
```

## Next Steps

- Read the [Quick Start Guide](getting-started.md) to learn basic usage
- Check the [SQL Reference Manual](sql-reference.md) to learn SQL syntax
- Refer to the [Configuration Guide](configuration.md) for advanced configuration
